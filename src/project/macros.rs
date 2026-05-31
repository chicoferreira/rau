#[macro_export]
macro_rules! resource_setters {
    (
        increases: $revisions:tt;
        $(
            pub fn $setter:ident($field:ident: $ty:ty);
        )*
    ) => {
        $(
            pub fn $setter(&mut self, $field: $ty) {
                if self.$field != $field {
                    self.$field = $field;
                    resource_setters!(@increase self $revisions);
                }
            }
        )*
    };
    (@increase $self:ident [$($revision:ident),* $(,)?]) => {
        $(
            $self.$revision.increase();
        )*
    };
}

#[macro_export]
macro_rules! resource_getters {
    () => {};
    (
        pub fn $getter:ident() -> &$ty:ty;
        $($rest:tt)*
    ) => {
        pub fn $getter(&self) -> &$ty {
            &self.$getter
        }

        resource_getters! {
            $($rest)*
        }
    };
    (
        pub fn $getter:ident() -> Option<&$ty:ty>;
        $($rest:tt)*
    ) => {
        pub fn $getter(&self) -> Option<&$ty> {
            self.$getter.as_ref()
        }

        resource_getters! {
            $($rest)*
        }
    };
    (
        pub fn $getter:ident() -> $ty:ty;
        $($rest:tt)*
    ) => {
        pub fn $getter(&self) -> $ty {
            self.$getter
        }

        resource_getters! {
            $($rest)*
        }
    };
}
