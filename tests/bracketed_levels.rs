use logsift::classify_level;
#[test]
fn common_log_level_delimiters_are_recognized() {
 for (line,level) in [("[INFO] started","INFO"),("ERROR: failed","ERROR"),("[WARN]: retry","WARN")] {
  assert_eq!(classify_level(line),Some(level));
 }
 for line in ["ERRORS:","[INFORMATION]","prefixERROR:","x[ERROR]x"] {assert_eq!(classify_level(line),None);}
}
