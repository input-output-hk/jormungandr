//! centralized styles for the different items to display.

#![allow(non_upper_case_globals)]

use console::Style;

pub mod icons {
    //! icons are not meant to add some SWAG to the display, not to use as
    //! a reference items. When using Icons, remember some users don't have
    //! unicode compatible/enabled terminals so always use this to _decorate_.

    pub type Icon = console::Emoji<'static, 'static>;

    lazy_static! {
        /// icon associated the test `Seed` to re-run the same tests with
        /// the same cryptographic materials.
        pub static ref seed: Icon = Icon::new("\u{1f331}", "");

        /// icon associated the `jcli` command line binary
        pub static ref jcli: Icon = Icon::new("\u{1f6e0} ", "");

        /// icon associated the `jcli` command line binary
        pub static ref jormungandr: Icon = Icon::new("\u{1f40d}", "");

        pub static ref wallet: Icon = Icon::new("\u{1f45b}", "");

        pub static ref account: Icon = Icon::new("\u{1f4b3}", "");

        pub static ref transaction: Icon = Icon::new("\u{1f9fe}", "");

        pub static ref block: Icon = Icon::new("\u{1f4e6}", "");

        pub static ref blockchain: Icon = Icon::new("\u{26d3}", "");

        pub static ref success: Icon = Icon::new("\u{2714}", "success");

        pub static ref failure: Icon = Icon::new("\u{1f4a5}", "failure");

        pub static ref ignored: Icon = Icon::new("\u{1f507}", "ignored");

    }
}

lazy_static! {
    /// style to apply when displaying the path to a binary/executable
    pub static ref binary: Style = Style::new().cyan();

    /// style to apply when displaying the `Seed`
    pub static ref seed: Style = Style::new().red();
}
