#[inline(always)]
pub fn report_line_err() {
    eprintln!("{}", line!());
}
