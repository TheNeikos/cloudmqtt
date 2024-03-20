macro_rules! speclink {
    ($anker:literal) => {
        std::concat!(
            "https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#",
            $anker
        )
    };
}
pub(crate) use speclink;

macro_rules! md_speclink {
    ($anker:literal) => {
        std::concat!(
            "[📖 Specification](",
            $crate::v5::util::speclink!($anker),
            ")"
        )
    };
}
pub(crate) use md_speclink;
