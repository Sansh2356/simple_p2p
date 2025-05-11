
//COMPILING THE CAPNP file .
// pub mod client;
// use std::{
//     env,
//     path::{Path, PathBuf},
// };

// fn main() {
//     println!("cargo:rerun-if-changed=capnp");
//     let out_dir = PathBuf::from(env::var("OUT_DIR").expect("Missing OUT_DIR"));
//     let capnp_dir = Path::new("");

//     let schemas = ["/home/sansh2356/Desktop/simple_p2p/src/test.capnp"];
//     let mut cmd = capnpc::CompilerCommand::new();
//     cmd.src_prefix(capnp_dir).output_path(&out_dir);
//     for schema in &schemas {
//         cmd.file(capnp_dir.join(schema));
//     }
//     cmd.run().expect("capnpc compilation failed");

// }



pub mod client;
pub mod addressbook {
    use capnp::serialize_packed;
    // use crate::client::test_capnp::{address_book, person};

    use super::client::test_capnp::{address_book, person};

    pub fn write_address_book() -> ::capnp::Result<()> {
        let mut message = ::capnp::message::Builder::new_default();

        let address_book = message.init_root::<address_book::Builder>();
        let mut people = address_book.init_people(2);

        let mut alice = people.reborrow().get(0);
        alice.set_id(123);
        alice.set_name("Alice");
        alice.set_email("alice@example.com");

        let mut alice_phones = alice.reborrow().init_phones(1);
        alice_phones.reborrow().get(0).set_number("555-1212");
        alice_phones
            .reborrow()
            .get(0)
            .set_type(person::phone_number::Type::Mobile);

        alice.get_employment().set_school("MIT");

        let mut bob = people.get(1);
        bob.set_id(456);
        bob.set_name("Bob");
        bob.set_email("bob@example.com");

        let mut bob_phones = bob.reborrow().init_phones(2);
        bob_phones.reborrow().get(0).set_number("555-4567");
        bob_phones
            .reborrow()
            .get(0)
            .set_type(person::phone_number::Type::Home);
        bob_phones.reborrow().get(1).set_number("555-7654");
        bob_phones
            .reborrow()
            .get(1)
            .set_type(person::phone_number::Type::Work);

        bob.get_employment().set_unemployed(());

        serialize_packed::write_message(&mut ::std::io::stdout(), &message)
    }

    pub fn print_address_book() -> ::capnp::Result<()> {
        let stdin = ::std::io::stdin();
        let message_reader = serialize_packed::read_message(
            &mut stdin.lock(),
            ::capnp::message::ReaderOptions::new(),
        )?;
        let address_book = message_reader.get_root::<address_book::Reader>()?;

        for person in address_book.get_people()? {
            println!(
                "{}: {}",
                person.get_name()?.to_str()?,
                person.get_email()?.to_str()?
            );
            for phone in person.get_phones()? {
                let type_name = match phone.get_type() {
                    Ok(person::phone_number::Type::Mobile) => "mobile",
                    Ok(person::phone_number::Type::Home) => "home",
                    Ok(person::phone_number::Type::Work) => "work",
                    Err(::capnp::NotInSchema(_)) => "UNKNOWN",
                };
                println!("  {} phone: {}", type_name, phone.get_number()?.to_str()?);
            }
            match person.get_employment().which() {
                Ok(person::employment::Unemployed(())) => {
                    println!("  unemployed");
                }
                Ok(person::employment::Employer(employer)) => {
                    println!("  employer: {}", employer?.to_str()?);
                }
                Ok(person::employment::School(school)) => {
                    println!("  student at: {}", school?.to_str()?);
                }
                Ok(person::employment::SelfEmployed(())) => {
                    println!("  self-employed");
                }
                Err(::capnp::NotInSchema(_)) => {}
            }
        }
        Ok(())
    }
}

pub fn main() {
    let args: Vec<String> = ::std::env::args().collect();
    if args.len() < 2 {
        println!("usage: $ {} [write | read]", args[0]);
    } else {
        match &*args[1] {
            "write" => addressbook::write_address_book().unwrap(),
            "read" => addressbook::print_address_book().unwrap(),
            _ => {
                println!("unrecognized argument")
            }
        }
    }
}
