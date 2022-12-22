use snafu::prelude::*;

#[derive(Debug, Snafu)]
struct Inner;

#[derive(Debug, Snafu)]
struct Error {
    source: Inner,
    name: String,
}

fn r() -> Result<(), Inner> {
    Ok(())
}

fn function_call() {
    fn make_name() -> String {
        "hi".into()
    }

    let _ = r().context(Snafu { name: make_name() });
}

fn method_call() {
    let name = "hi";

    let _ = r().context(Snafu {
        name: name.to_string(),
    });
}

fn closure_call() {
    let make_name = || String::from("hi");

    let _ = r().context(Snafu { name: make_name() });
}

#[allow(unused_parens)]
fn nested_call() {
    let _ = r().context(Snafu {
        name: ({ ({ "hi".to_string() }) }),
    });
}

fn context_method_not_from_snafu() {
    #[allow(dead_code)]
    struct Args {
        name: String,
    }
    struct NotResult;
    trait NotResultExt {
        fn context(self, arg: Args);
    }
    impl NotResultExt for NotResult {
        fn context(self, _: Args) {}
    }

    NotResult.context(Args {
        name: "hi".to_string(),
    });
}

fn allows_some_calls() {
    let string = String::from("hi");

    // Inherent method as a method
    let _ = r().context(Snafu {
        name: string.as_str(),
    });

    // Inherent method as a function
    let _ = r().context(Snafu {
        name: String::as_str(&string),
    });

    // Trait method as a function of trait object
    let _ = r().context(Snafu {
        name: <dyn AsRef<str>>::as_ref(&string),
    });

    // Trait method as a function of concrete type
    let _ = r().context(Snafu {
        name: <_ as AsRef<str>>::as_ref(&string),
    });

    let box_string = Box::new(String::from("hi"));

    // Inherent method through deref
    let _ = r().context(Snafu {
        name: box_string.as_str(),
    });

    // Trait method through deref
    let _ = r().context(Snafu {
        name: box_string.as_ref(),
    });

    fn check_opaque(opaque_as_ref: impl AsRef<str>) {
        // Trait method as method
        let _ = r().context(Snafu {
            name: opaque_as_ref.as_ref(),
        });

        // Trait method as function
        let _ = r().context(Snafu {
            name: <_ as AsRef<str>>::as_ref(&opaque_as_ref),
        });
    }
    check_opaque(&string);

    struct SimilarMethods;
    impl SimilarMethods {
        fn as_str(&self) -> &str {
            "hi "
        }
    }

    let _ = r().context(Snafu {
        name: SimilarMethods.as_str(),
    });

    let _ = r().context(Snafu {
        name: SimilarMethods::as_str(&SimilarMethods),
    });
}

fn context_from_all_extension_traits() {
    fn o() -> Option<()> {
        Some(())
    }

    fn make_name() -> String {
        "hi".into()
    }

    #[derive(Debug, Snafu)]
    struct MissingError {
        name: String,
    }

    let _ = o().context(MissingSnafu { name: make_name() });

    async fn ra() -> Result<(), Inner> {
        Ok(())
    }

    let _ = ra().context(Snafu { name: make_name() });

    let _ = futures::stream::once(ra()).context(Snafu { name: make_name() });
}

// whatever_context

fn main() {
    function_call();
    method_call();
    closure_call();
    nested_call();
    context_method_not_from_snafu();
    allows_some_calls();
    context_from_all_extension_traits();
}
