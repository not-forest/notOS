// The iterable two-side enum trait. The trait itself must have Clone, Copy, PartialEq, Eq derives,
// to work correctly.

pub trait IternumTrait: Sized {
    // The overall size of enum.
    const SIZE: usize;
    // This function creates an array of variants where each item is a reference to a variant
    fn iter() -> [Self; Self::SIZE];
    // Returns an index of the variant inside the enum. If it does not exist returns usize::MAX.
    fn get_index(variant: Self) -> usize;
    // Returns the variant that is located at corresponding index.
    fn get_variant(index: usize) -> Self;
    // Returns the total amount of variants in enum
    fn get_size() -> usize;
    // TODO! Enum's constructors handling will be added in the future.
}

#[test_case]
fn test_enum_iterator() {
    use proc_macros::Iternum;

    #[derive(Iternum, Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
    pub enum TestEnum {
        This, Is, A, TEST,
    }

    // Gets variant by index, while index is acquired my variant's name.
    let variant = TestEnum::get_variant(TestEnum::get_index(TestEnum::TEST));

    assert_eq!(TestEnum::TEST, variant);
}
