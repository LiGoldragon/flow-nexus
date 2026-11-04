fn integer(value: Option<String>, name: &str) -> Result<i64, Box<dyn std::error::Error>> { value.ok_or_else(|| name.to_owned())?.parse().map_err(Into::into) }
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let name = args.next().ok_or("METAFLOW")?;
    let text = args.next().ok_or("DATOM")?;
    let value = flow_nexus_wire::AdmissionValue { frame_deadline_milliseconds: integer(args.next(), "DEADLINE")?, frame_capacity_bytes: integer(args.next(), "CAPACITY")?, retry_delay_milliseconds: integer(args.next(), "RETRY")? };
    if args.next().is_some() { return Err("unexpected CLI argument".into()); }
    let admission = flow_nexus_wire::validate_admission(&value)?;
    let place = shared_translator::placement::resolve_place(&name)?;
    println!("{}", flow_nexus_translator::request(&place, admission, &text)?);
    Ok(())
}
