use crate::analysis::{DailyStats, FieldStats};
use crate::data;

/// Shows the minimum, maximum and averages for today's saved readings.
pub fn show_statistics() {
    let path = data::csv_path();

    let rows = match data::read_samples(&path) {
        Ok(rows) => rows,
        Err(error) => {
            println!("Could not read {}: {error}", path.display());
            return;
        }
    };

    let today = chrono::Local::now().format("%Y-%m-%d").to_string();

    let samples: Vec<_> = rows
        .iter()
        .filter(|row| row.date == today)
        .map(|row| row.sample.clone())
        .collect();

    println!("Daily statistics for {today}");
    println!();

    let stats = match DailyStats::from_samples(&samples) {
        Some(stats) => stats,
        None => {
            println!("No readings have been saved for today yet.");
            println!();
            println!("Use option 5 to start saving readings.");
            return;
        }
    };

    println!("Saved readings today: {}", stats.readings);
    println!();

    println!(
        "{:<14}{:>7} {:>9} {:>9} {:>10} {:>14} {:>9}",
        "Reading", "Count", "Min", "Max", "Average", "Avg (no outl.)", "Outliers"
    );

    print_row("Temperature", stats.temperature.as_ref());
    print_row("Humidity (%)", stats.humidity.as_ref());
    print_row("Pressure (Pa)", stats.pressure.as_ref());
    print_row("Light (raw)", stats.sunlight.as_ref());

    println!();
    println!("Outliers are readings more than two standard deviations from the average.");
}

fn print_row(name: &str, stats: Option<&FieldStats>) {
    match stats {
        Some(stats) => println!(
            "{:<14}{:>7} {:>9.2} {:>9.2} {:>10.2} {:>14.2} {:>9}",
            name,
            stats.count,
            stats.min,
            stats.max,
            stats.average,
            stats.average_without_outliers,
            stats.outliers_removed
        ),
        None => println!("{name:<14}{:>7}", "-"),
    }
}

/// Shows the most recent rows of the saved data as a table.
pub fn show_spreadsheet() {
    let path = data::csv_path();

    let rows = match data::read_samples(&path) {
        Ok(rows) => rows,
        Err(error) => {
            println!("Could not read {}: {error}", path.display());
            return;
        }
    };

    if rows.is_empty() {
        println!("No readings have been saved yet.");
        println!();
        println!("Use option 5 to start saving readings.");
        return;
    }

    println!("{} saved readings in {}", rows.len(), path.display());
    println!();

    println!(
        "{:<20}{:>11}{:>14}{:>14}{:>9}",
        "Timestamp", "Temp (°C)", "Humidity (%)", "Pressure (Pa)", "Light"
    );

    // Only the newest rows are printed so the table stays readable.
    const SHOWN_ROWS: usize = 25;
    let start = rows.len().saturating_sub(SHOWN_ROWS);

    if start > 0 {
        println!("... {start} earlier rows not shown");
    }

    for row in &rows[start..] {
        println!(
            "{:<20}{:>11}{:>14}{:>14}{:>9}",
            row.timestamp,
            number(row.sample.temperature),
            number(row.sample.humidity),
            number(row.sample.pressure),
            whole(row.sample.visible_light)
        );
    }
}

/// Explains what the project does, what UV is and why the device has no UV
/// reading.
pub fn show_information() {
    println!("CaseyShade");
    println!("==========");
    println!();
    println!("CaseyShade is a portable environmental monitor. It reads temperature,");
    println!("humidity, air pressure and visible light, saves each reading with the");
    println!("date and time, shows the readings as graphs, and can send the saved");
    println!("data to a computer over Wi-Fi.");
    println!();
    println!("About UV");
    println!("--------");
    println!("Ultraviolet radiation comes from the sun. Too much exposure causes");
    println!("sunburn, eye damage and, over many years, skin cancer. The UV Index");
    println!("describes how strong the UV is:");
    println!();
    println!("  0-2   low        little risk");
    println!("  3-5   moderate   cover up around midday");
    println!("  6-7   high       sunscreen, hat and shade");
    println!("  8-10  very high  avoid the sun around midday");
    println!("  11+   extreme    stay inside where possible");
    println!();
    println!("Why this device does not show a UV Index");
    println!("---------------------------------------");
    println!("The light sensor fitted to CaseyShade is a Silicon Labs Si1151. It has");
    println!("visible and infrared photodiodes but no UV-sensitive channel, so it");
    println!("cannot measure UV. Its visible-light channel is used for the light");
    println!("readings. Measuring UV would need a dedicated UV sensor such as an");
    println!("Si1133 or VEML6075.");
}

fn number(value: Option<f32>) -> String {
    match value {
        Some(value) => format!("{value:.2}"),
        None => "-".to_string(),
    }
}

fn whole(value: Option<u16>) -> String {
    match value {
        Some(value) => value.to_string(),
        None => "-".to_string(),
    }
}
