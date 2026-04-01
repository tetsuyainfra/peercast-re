//
macro_rules! merge_ref {
    // merge_ref(self, other, prop)
    ($self: ident, $other: ident, $prop: ident) => {
        $crate::pcp::atom::decode::macros::_merge_ref(&mut ($self.$prop), &($other.$prop))
    };
    // merge_ref(self, other, [prop1, prop2])
    ($self: ident, $other: ident, [ $($prop:ident),* ]) => {
        {
            let mut changed = false;
            $(
               changed = $crate::pcp::atom::decode::macros::_merge_ref(&mut ($self.$prop), &($other.$prop)) || changed;
            )*
            changed
        }
    };
}
pub(crate) use merge_ref;

pub(crate) fn _merge_ref<T: Clone>(x: &mut Option<T>, y: &Option<T>) -> bool {
    if y.is_none() {
        return false;
    } else {
        *x = y.clone();
        true
    }
}

macro_rules! getter {
    (&$self: ident, $prop: ident) => {
        pub fn $prop(&$self) -> String {
            if let Some(ref $prop) = $self.$prop {
                $prop.clone()
            } else {
                String::from("")
            }
        }
    };
    (&$self: ident, $prop: ident, $TYPE: ty, $DEF_VAL: expr) => {
        pub fn $prop(&$self) -> $TYPE {
            if let Some(ref $prop) = $self.$prop {
                $prop.clone()
            } else {
                $DEF_VAL
            }
        }
    };
}
pub(crate) use getter;

#[cfg(test)]
mod t {

    #[test]
    fn test_merge_ref() {
        #[derive(Clone, PartialEq, Eq, Debug)]
        struct S {
            a: Option<bool>,
            b: Option<u32>,
        }
        let s1 = S { a: None, b: None };
        let s2 = S {
            a: Some(true),
            b: Some(1),
        };

        let mut c1 = s1.clone();
        assert_eq!(merge_ref!(c1, s2, a), true);
        assert_eq!(
            c1,
            S {
                a: Some(true),
                b: None,
            }
        );

        let mut c2 = c1.clone();
        assert_eq!(merge_ref!(c2, s1, a), false);
        assert_eq!(
            c2,
            S {
                a: Some(true),
                b: None
            }
        );

        let mut c3 = s1.clone();
        assert_eq!(merge_ref!(c3, s2, [a, b]), true);
        assert_eq!(
            c3,
            S {
                a: Some(true),
                b: Some(1)
            }
        );
    }

    #[ignore = "because simple"]
    #[test]
    fn test_or() {
        let mut changed = false;
        changed = false || changed;
        assert_eq!(changed, false);

        changed = true || changed;
        assert_eq!(changed, true);

        changed = false || changed;
        assert_eq!(changed, true);
    }
}
