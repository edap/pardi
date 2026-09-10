use anyhow::Error;

pub fn print_error_messages(err: Error, debug_flag: bool) {
    if debug_flag {
        eprintln!("{}", err);
    }
}
