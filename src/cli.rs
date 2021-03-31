use clap::{clap_app, crate_authors, crate_description, crate_name, crate_version};

pub(crate) fn setup_cli() -> clap::ArgMatches<'static> {
    clap_app!(myapp =>
        (name: crate_name!())
        (version: crate_version!())
        (author: crate_authors!())
        (about: crate_description!())
        (@arg config: -c --config +takes_value "Set config file path")
    )
    .get_matches()
}
