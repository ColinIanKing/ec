use ectool::{
    Ec,
    Error,
    Firmware,
    Timeout,
};
use std::{
    env,
    fs,
    io,
    process,
    str,
    time::{Duration, Instant},
    thread,
};

pub struct StdTimeout {
    instant: Instant,
    duration: Duration,
}

impl StdTimeout {
    pub fn new(duration: Duration) -> Self {
        StdTimeout {
            instant: Instant::now(),
            duration
        }
    }
}

impl Timeout for StdTimeout {
    fn reset(&mut self) {
        self.instant = Instant::now();
    }

    fn running(&self) -> bool {
        self.instant.elapsed() < self.duration
    }
}

unsafe fn iopl() {
    extern {
        fn iopl(level: isize) -> isize;
    }

    if iopl(3) < 0 {
        eprintln!("failed to get I/O permission: {}", io::Error::last_os_error());
        process::exit(1);
    }
}

unsafe fn console() -> Result<(), Error> {
    iopl();

    let mut ec = Ec::new(
        true,
        StdTimeout::new(Duration::new(1, 0)),
    )?;

    let mut head = ec.debug(0) as usize;
    loop {
        let tail = ec.debug(0) as usize;
        if tail == 0 || head == tail {
            thread::sleep(Duration::from_millis(1));
        } else {
            while head != tail {
                head += 1;
                if head >= 256 { head = 1; }
                let c = ec.debug(head as u8);
                print!("{}", c as char);
            }
        }
    }
}

unsafe fn flash(path: &str) -> Result<(), Error> {
    //TODO: remove unwraps
    let firmware_data = fs::read(path).unwrap();
    let firmware = Firmware::new(&firmware_data).unwrap();
    println!("file board: {:?}", str::from_utf8(firmware.board));
    println!("file version: {:?}", str::from_utf8(firmware.version));

    iopl();

    let mut ec = Ec::new(
        true,
        StdTimeout::new(Duration::new(1, 0)),
    )?;

    {
        let mut data = [0; 256];
        let size = ec.board(&mut data)?;

        let ec_board = &data[..size];
        println!("ec board: {:?}", str::from_utf8(ec_board));

        if ec_board != firmware.board {
            panic!("file board does not match ec board");
        }
    }

    {
        let mut data = [0; 256];
        let size = ec.version(&mut data)?;

        let ec_version = &data[..size];
        println!("ec version: {:?}", str::from_utf8(ec_version));
    }

    Ok(())
}

unsafe fn info() -> Result<(), Error> {
    iopl();

    let mut ec = Ec::new(
        true,
        StdTimeout::new(Duration::new(1, 0)),
    )?;

    {
        print!("board: ");
        let mut data = [0; 256];
        let size = ec.board(&mut data)?;
        for &b in data[..size].iter() {
            print!("{}", b as char);
        }
        println!();
    }

    {
        print!("version: ");
        let mut data = [0; 256];
        let size = ec.version(&mut data)?;
        for &b in data[..size].iter() {
            print!("{}", b as char);
        }
        println!();
    }
    
    Ok(())
}

fn usage() {
    eprintln!("  console");
    eprintln!("  flash [file]");
    eprintln!("  info");
}

fn main() {
    let mut args = env::args().skip(1);

    match args.next() {
        Some(arg) => match arg.as_str() {
            "console" => match unsafe { console() } {
                Ok(()) => (),
                Err(err) => {
                    eprintln!("failed to read console: {:X?}", err);
                    process::exit(1);
                },
            },
            "flash" => match args.next() {
                Some(path) => match unsafe { flash(&path) } {
                    Ok(()) => (),
                    Err(err) => {
                        eprintln!("failed to flash '{}': {:X?}", path, err);
                        process::exit(1);
                    },
                },
                None => {
                    eprintln!("no file provided");
                    process::exit(1);
                }
            },
            "info" => match unsafe { info() } {
                Ok(()) => (),
                Err(err) => {
                    eprintln!("failed to read info: {:X?}", err);
                    process::exit(1);
                },
            },
            _ => {
                eprintln!("unknown subcommand '{}'", arg);
                usage();
                process::exit(1);
            },
        },
        None => {
            eprintln!("no subcommand provided");
            usage();
            process::exit(1);
        },
    }

}