/// A structure that contains bitflags. All bit-like operations are available.

use core::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Not, Deref, DerefMut};

/// A single node reference to a bitflag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct BitNode<T>(pub T);

impl<T: Copy + Clone> BitNode<T> where 
    T: PartialEq,
    T: BitAnd,
    T: BitOr,
{
    /// Checks if the specific bitflag exists in given bits
    pub fn is_in(&self, bits: T) -> bool where 
        <T as BitAnd>::Output: PartialEq<T>
    {
        bits & **self == **self
    }
}

impl<T> From<T> for BitNode<T> {
    fn from(value: T) -> Self {
        BitNode(value)
    }
}

impl From<BitNode<u16>> for u16 {
    fn from(value: BitNode<u16>) -> Self {
        value.0
    }
}

impl From<BitNode<u32>> for u32 {
    fn from(value: BitNode<u32>) -> Self {
        value.0
    }
}

impl From<BitNode<u64>> for u64 {
    fn from(value: BitNode<u64>) -> Self {
        value.0
    }
}

impl From<BitNode<u128>> for u128 {
    fn from(value: BitNode<u128>) -> Self {
        value.0
    }
}

impl From<BitNode<usize>> for usize {
    fn from(value: BitNode<usize>) -> Self {
        value.0
    }
}

impl<T> Deref for BitNode<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for BitNode<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

impl<T: Copy + Clone> BitAnd for BitNode<T> where T: BitAnd<Output = T> {
    type Output = BitNode<T>;
    fn bitand(self, rhs: Self) -> Self::Output {
        BitNode( *self & *rhs )
    }
}

impl<T: Copy + Clone> BitAndAssign for BitNode<T> where T: BitAnd<Output = T> {
    fn bitand_assign(&mut self, rhs: Self) {
        *self = *self & rhs
    }
}

impl<T: Copy + Clone> BitOr for BitNode<T> where T: BitOr<Output = T> {
    type Output = BitNode<T>;
    fn bitor(self, rhs: Self) -> Self::Output {
        BitNode( *self | *rhs )
    }
}

impl<T: Copy + Clone> BitOrAssign for BitNode<T> where T: BitOr<Output = T> {
    fn bitor_assign(&mut self, rhs: Self) {
        *self = *self | rhs
    }
}

impl<T: Copy + Clone> BitXor for BitNode<T> where T: BitXor<Output = T> {
    type Output = BitNode<T>;
    fn bitxor(self, rhs: Self) -> Self::Output {
        BitNode( *self ^ *rhs )
    }
}

impl<T: Copy + Clone> BitXorAssign for BitNode<T> where T: BitXor<Output = T> {
    fn bitxor_assign(&mut self, rhs: Self) {
        *self = *self ^ rhs
    }
}

impl<T: Copy + Clone> Not for BitNode<T> where T: Not<Output = T> {
    type Output = BitNode<T>;
    fn not(self) -> Self::Output {
        BitNode( !*self )
    }
}

/// This macro exports bitflags provided, while also creating an enum
/// that will consist of 'BitNode's with values.
#[macro_export]
macro_rules! bitflags {
    (
        $(#[$meta:meta])*
        pub struct $name:ident: $underlying:ty {
            $(
                $(#[$flag_meta:meta])*
                const $flag:ident = $bit:expr,
            )*
        };
    ) => {
        bitflags! {
            $(#[$meta])*
            pub struct $name: $underlying {
                $(
                    $(#[$flag_meta])*
                    const $flag = $bit,
                )*
            }
        }
    };

    (
        $(
            $(#[$meta:meta])*
            pub struct $name:ident: $underlying:ty {
                $(
                    $(#[$flag_meta:meta])*
                    const $flag:ident = $bit:expr,
                )*
            };
        )*
    ) => {
        $(
            bitflags! {
                $(#[$meta])*
                pub struct $name: $underlying {
                    $(
                        $(#[$flag_meta])*
                        const $flag = $bit,
                    )*
                }
            }
        )*
    };
    
    (
        $(#[$meta:meta])*
        pub struct $name:ident: $underlying:ty {
            $(
                $(#[$flag_meta:meta])*
                const $flag:ident = $bit:expr,
            )*
        }
    ) => {
        crate::_inner_bitflags! {
            $(#[$meta])*
            pub struct $name: $underlying {
                $(
                    $(#[$flag_meta])*
                    const $flag = $bit,
                )*
            }
        }
    };

    () => ();
}

#[doc(hidden)]
#[macro_export]
macro_rules! _inner_bitflags {
    (
        $(#[$meta:meta])*
        pub struct $name:ident: $underlying:ty {
            $(
                $(#[$flag_meta:meta])*
                const $flag:ident = $bit:expr,
            )*
        }
    ) => {
        $(#[$meta])*
        #[allow(non_camel_case_types)]
        #[repr($underlying)]
        pub enum $name {
            $(
                $(#[$flag_meta])*
                $flag,
            )*
            Custom($underlying),
            EMPTY,
            ALL,
        }

        impl $name {
            pub fn is_in(&self, bits: $underlying) -> bool {
                self.as_node().is_in(bits)
            }

            pub fn empty() -> Self {
                Self::EMPTY
            }
        
            pub fn all() -> Self {
                Self::ALL
            }

            pub fn bits(&self) -> $underlying {
                <$underlying>::from(*self)
            }

            pub fn from_bits_truncate(bits: $underlying) -> crate::kernel_components::structures::BitNode<$underlying> {
                ($name::Custom(bits) & Self::all()).into()
            }

            pub fn as_node(&self) -> crate::kernel_components::structures::BitNode<$underlying> {
                crate::kernel_components::structures::BitNode((*self as $name).into())
            }
        }

        impl From<$underlying> for $name {
            fn from(value: $underlying) -> Self {
                $name::Custom(value)
            }
        }

        impl From<$name> for $underlying {
            fn from(value: $name) -> Self {
                match value {
                    $name::Custom(n) => n,
                    $name::ALL => <$underlying>::MAX,
                    $name::EMPTY => 0 as $underlying,
                    $(
                        $(#[$flag_meta])*
                        $name::$flag => $bit,
                    )*
                }
            }
        }

        impl From<$name> for crate::kernel_components::structures::BitNode<$underlying> {
            fn from(value: $name) -> Self {
                crate::kernel_components::structures::BitNode(<$underlying>::from(value))
            }
        }

        impl From<crate::kernel_components::structures::BitNode<$underlying>> for $name {
            fn from(value: crate::kernel_components::structures::BitNode<$underlying>) -> Self {
                $name::Custom(value.0)
            }
        }

        impl core::ops::BitAnd for $name {
            type Output = Self;
            fn bitand(self, rhs: Self) -> Self::Output {
                $name::Custom(<$underlying>::from(self) & <$underlying>::from(rhs))
            }
        }

        impl core::ops::BitOr for $name {
            type Output = Self;
            fn bitor(self, rhs: Self) -> Self::Output {
                $name::Custom(<$underlying>::from(self) | <$underlying>::from(rhs))
            }
        }

        impl core::ops::BitXor for $name {
            type Output = Self;
            fn bitxor(self, rhs: Self) -> Self::Output {
                $name::Custom(<$underlying>::from(self) ^ <$underlying>::from(rhs))
            }
        }

        impl core::ops::Not for $name {
            type Output = Self;
            fn not(self) -> Self::Output {
                $name::Custom(!<$underlying>::from(self))
            }
        }
    };
}