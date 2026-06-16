use std::process::ExitCode;

use ubid::{Ubid40, Ubid80, Ubid120, Ubid160};

fn main() -> ExitCode {
    let mut status = ExitCode::SUCCESS;
    let mut saw_arg = false;

    for arg in std::env::args().skip(1) {
        saw_arg = true;
        let ubid = match arg.as_str() {
            "40" => Ubid40::generate().encode(),
            "80" => Ubid80::generate().encode(),
            "120" => Ubid120::generate().encode(),
            "160" => Ubid160::generate().encode(),
            _ => {
                eprintln!("unsupported width `{arg}`; expected one of: 40, 80, 120, 160");
                status = ExitCode::FAILURE;
                continue;
            }
        };
        println!("{ubid}");
    }

    if !saw_arg {
        eprintln!("usage: ubidgen <40|80|120|160>...");
        return ExitCode::FAILURE;
    }

    status
}
