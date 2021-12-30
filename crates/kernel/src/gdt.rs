use x86_64::{
    registers::segmentation::SegmentSelector,
    structures::{
        gdt::{Descriptor, GlobalDescriptorTable},
        tss::TaskStateSegment,
    },
    VirtAddr,
};

pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;

lazy_static! {
    static ref TSS: TaskStateSegment = {
        let mut tss = TaskStateSegment::new();
        tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = {
            const STACK_SIZE: usize = 4096 * 5;
            static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

            let stack_start = VirtAddr::from_ptr(unsafe { &STACK });
            stack_start + STACK_SIZE
        };
        tss
    };
}
lazy_static! {
    static ref GDT: (GlobalDescriptorTable, Selectors) = {
        let mut gdt = GlobalDescriptorTable::new();
        let code_selector = gdt.add_entry(Descriptor::kernel_code_segment());
        let tss_selector = gdt.add_entry(Descriptor::tss_segment(&TSS));
        let stack_selector = gdt.add_entry(Descriptor::kernel_data_segment());
        (
            gdt,
            Selectors {
                code_selector,
                tss_selector,
                stack_selector,
            },
        )
    };
}
struct Selectors {
    code_selector: SegmentSelector,
    tss_selector: SegmentSelector,
    stack_selector: SegmentSelector,
}

pub fn initialize() {
    use x86_64::instructions::segmentation::{Segment, CS, SS};
    GDT.0.load();
    let selectors = &GDT.1;
    unsafe {
        CS::set_reg(selectors.code_selector);
        x86_64::instructions::tables::load_tss(selectors.tss_selector);
        SS::set_reg(selectors.stack_selector);
    }
}
