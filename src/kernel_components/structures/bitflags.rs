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
        }
    ) => {
        use core::ops::{BitAnd, BitOr, BitXor, Not};
        use crate::kernel_components::structures::BitNode;
        use $name::*;

        $(#[$meta])*
        #[derive(Debug, Hash)]
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

            pub fn from_bits_truncate(bits: $underlying) -> BitNode<$underlying> {
                (Custom(bits) & Self::all()).into()
            }

            pub fn as_node(&self) -> BitNode<$underlying> {
                BitNode((*self as $name).into())
            }
        }

        impl From<$underlying> for BitNode<$underlying> {
            fn from(value: $underlying) -> Self {
                BitNode(value)
            }
        }
        
        impl From<BitNode<$underlying>> for $underlying {
            fn from(value: BitNode<$underlying>) -> Self {
                value.0
            }
        }

        impl From<$underlying> for $name {
            fn from(value: $underlying) -> Self {
                Custom(value)
            }
        }

        impl From<$name> for $underlying {
            fn from(value: $name) -> Self {
                match value {
                    Custom(n) => n,
                    ALL => <$underlying>::MAX,
                    EMPTY => 0 as $underlying,
                    $(
                        $(#[$flag_meta])*
                        $flag => $bit,
                    )*
                }
            }
        }

        impl From<$name> for BitNode<$underlying> {
            fn from(value: $name) -> Self {
                BitNode(<$underlying>::from(value))
            }
        }

        impl From<BitNode<$underlying>> for $name {
            fn from(value: BitNode<$underlying>) -> Self {
                Custom(value.0)
            }
        }

        impl BitAnd for $name {
            type Output = Self;
            fn bitand(self, rhs: Self) -> Self::Output {
                Custom(<$underlying>::from(self) & <$underlying>::from(rhs))
            }
        }

        impl BitOr for $name {
            type Output = Self;
            fn bitor(self, rhs: Self) -> Self::Output {
                Custom(<$underlying>::from(self) | <$underlying>::from(rhs))
            }
        }
    };
}
