use std::fs::File;
use std::io;
use std::path::Path;

///
/// Ayxilary function to create a CSV of desired number of rows.
///
pub fn write_csv<P: AsRef<Path>>(path: P) -> io::Result<()> {
    let file = File::create(path)?;
    let mut wtr = csv::Writer::from_writer(file);

    // Since we're writing records manually, we must explicitly write our
    // header record. A header record is written the same way that other
    // records are written.
    wtr.write_record(["type", "client", "tx", "amount"])?;
    wtr.write_record(["deposit", "1", "1", "1.0"])?;
    wtr.write_record(["deposit", "2", "2", "2.0"])?;
    wtr.write_record(["deposit", "1", "3", "2.0"])?;
    wtr.write_record(["withdrawal", "1", "4", "1.5"])?;
    wtr.write_record(["withdrawal", "2", "5", "3.0"])?;

    // A CSV writer maintains an internal buffer, so it's important
    // to flush the buffer when you're done.
    wtr.flush()?;
    Ok(())
}
