use capnp::message;
use demo::demo_capnp;

fn main() -> capnp::Result<()> {
    // Test that we can actually create and use the generated types
    let mut message = message::Builder::new_default();

    // Create a Person
    let mut person = message.init_root::<demo_capnp::person::Builder>();
    person.set_id(12345);
    person.set_full_name("John Doe");
    person.set_age(30);
    person.set_is_active(true);
    person.set_score(95.5);

    // Set email addresses
    let mut emails = person.reborrow().init_email_addresses(2);
    emails.set(0, "john@example.com");
    emails.set(1, "john.doe@work.com");

    // Set tags
    let mut tags = person.reborrow().init_tags(3);
    tags.set(0, "engineer");
    tags.set(1, "rust");
    tags.set(2, "capnproto");

    // Set status to Active
    person.reborrow().get_status()?.set_active(());

    println!("Successfully created a Person with Cap'n Proto!");
    println!("Person ID: {}", person.reborrow().get_id());
    println!(
        "Person name: {}",
        person.reborrow().get_full_name()?.to_str()?
    );

    Ok(())
}