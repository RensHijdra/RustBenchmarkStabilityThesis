// use std::fs::File;
// use std::io::{BufRead, BufReader};
//
// #[derive(Debug)]
// struct ProfileDataFile {
//     format_spec: Option<String>,
//     format_version: Option<String>,
//     creator: Option<String>,
//     part_data: Vec<PartData>,
// }
//
// #[derive(Debug)]
// enum PartData {
//     HeaderLine(String),
//     BodyLines(Vec<String>),
//     PartDetail(PartDetail),
// }
//
// #[derive(Debug)]
// enum PartDetail {
//     TargetCommand(String),
//     TargetID(TargetID),
//     Description(String, String),
//     EventSpecification(String, Option<String>, Option<String>),
//     CostLineDef(String, Option<String>),
// }
//
// #[derive(Debug)]
// enum TargetID {
//     Pid(String),
//     Thread(String),
//     Part(String),
// }
//
// pub fn parse_file(path: &str) -> Result<ProfileDataFile, std::io::Error> {
//     let file = File::open(path)?;
//     let reader = BufReader::new(file);
//     let mut profile_data_file = ProfileDataFile {
//         format_spec: None,
//         format_version: None,
//         creator: None,
//         part_data: Vec::new(),
//     };
//
//     let mut current_part_data: Option<Vec<PartData>> = None;
//     let mut current_body_lines: Option<Vec<String>> = None;
//
//     for line_result in reader.lines() {
//         let line = line_result?;
//
//         if line.is_empty() {
//             // Empty line
//             if let Some(part_data) = current_part_data.take() {
//                 profile_data_file.part_data.push(part_data);
//             }
//         } else if line.starts_with('#') {
//             // Comment line or header line
//             if let Some(part_data) = current_part_data.as_mut() {
//                 part_data.push(parse_header_line(&line));
//             }
//         } else if let Some(body_lines) = current_body_lines.as_mut() {
//             // Body line
//             body_lines.push(line.to_owned());
//         } else {
//             // Part data
//             current_body_lines = Some(Vec::new());
//             current_body_lines.as_mut().unwrap().push(line.to_owned());
//             current_part_data = Some(PartData::BodyLines(current_body_lines.take().unwrap()));
//         }
//     }
//
//     if let Some(part_data) = current_part_data {
//         profile_data_file.part_data.push(part_data);
//     }
//
//     Ok(profile_data_file)
// }
//
// fn parse_header_line(line: &str) -> PartData {
//     if line.trim().is_empty() {
//         PartData::HeaderLine(line.to_owned())
//     } else {
//         PartData::HeaderLine(parse_part_detail(line))
//     }
// }
//
// fn parse_part_data(lines: Vec<String>) -> PartData {
//     if lines.is_empty() {
//         return PartData::BodyLines(lines);
//     }
//
//     let first_line = lines[0].clone();
//     if first_line.starts_with("cmd:") {
//         PartData::PartDetail(PartDetail::TargetCommand(parse_target_command(&first_line)))
//     } else if first_line.starts_with("pid:") {
//         PartData::PartDetail(PartDetail::TargetID(parse_target_id(&first_line, "pid")))
//     } else if first_line.starts_with("thread:") {
//         PartData::PartDetail(PartDetail::TargetID(parse_target_id(&first_line, "thread")))
//     } else if first_line.starts_with("part:") {
//         PartData::PartDetail(PartDetail::TargetID(parse_target_id(&first_line, "part")))
//     } else {
//         PartData::HeaderLine(parse_part_detail(&first_line))
//     }
// }
//
// fn parse_part_detail(line: &str) -> PartDetail {
//     if line.starts_with("cmd:") {
//         PartDetail::TargetCommand(parse_target_command(line))
//     } else if line.starts_with("pid:") {
//         PartDetail::TargetID(parse_target_id(line, "pid"))
//     } else if line.starts_with("thread:") {
//         PartDetail::TargetID(parse_target_id(line, "thread"))
//     } else if line.starts_with("part:") {
//         PartDetail::TargetID(parse_target_id(line, "part"))
//     } else if line.starts_with("desc:") {
//         let (name, description) = parse_description(line);
//         PartDetail::Description(name, description)
//     } else if line.starts_with("event:") {
//         let (name, inherited_def, long_name_def) = parse_event_specification(line);
//         PartDetail::EventSpecification(name, inherited_def, long_name_def)
//     } else if line.starts_with("events:") {
//         let (name, space_names) = parse_cost_line_def(line, "events:");
//         PartDetail::CostLineDef(name, space_names)
//     } else if line.starts_with("positions:") {
//         let (name, space_names) = parse_cost_line_def(line, "positions:");
//         PartDetail::CostLineDef(name, space_names)
//     } else {
//         panic!("Unexpected part detail line: {}", line);
//     }
// }
//
// fn parse_target_command(line: &str) -> String {
//     line.trim_start_matches("cmd:").trim().to_owned()
// }
//
// fn parse_target_id(line: &str, id_type: &str) -> TargetID {
//     let value = line.trim_start_matches(format!("{}:", id_type).as_str()).trim().to_owned();
//     match id_type {
//         "pid" => TargetID::Pid(value),
//         "thread" => TargetID::Thread(value),
//         "part" => TargetID::Part(value),
//         _ => panic!("Invalid target ID type: {}", id_type),
//     }
// }
//
// fn parse_description(line: &str) -> (String, String) {
//     let mut parts = line.trim_start_matches("desc:").splitn(2, ':');
//     let name = parts.next().unwrap().trim().to_owned();
//     let description = parts.next().unwrap().trim().to_owned();
//     (name, description)
// }
//
// fn parse_event_specification(line: &str) -> (String, Option<String>, Option<String>) {
//     let mut parts = line.trim_start_matches("event:").splitn(3, ' ');
//     let name = parts.next().unwrap().trim().to_owned();
//     let inherited_def = match parts.next() {
//         Some("=") => Some(parse_inherited_expr(parts.next().unwrap())),
//         Some(long_name_def) => Some(long_name_def.to_owned()),
//         None => None,
//     };
//     let long_name_def = parts.next().map(|s| s.to_owned());
//     (name, inherited_def, long_name_def)
// }
//
// fn parse_inherited_expr(expr: &str) -> String {
//     // Implementation for parsing the inherited expression goes here
//     expr.trim().to_owned()
// }
//
// fn parse_cost_line_def(line: &str, prefix: &str) -> (String, Option<String>) {
//     let mut parts = line.trim_start_matches(prefix).splitn(2, ' ');
//     let name = parts.next().unwrap().trim().to_owned();
//     let space_names = parts.next().map(|s| s.trim().to_owned());
//     (name, space_names)
// }
//
// fn main() {
//     match parse_file("path/to/your/file") {
//         Ok(profile_data_file) => {
//             println!("{:#?}", profile_data_file);
//         }
//         Err(e) => eprintln!("Error: {}", e),
//     }
// }
