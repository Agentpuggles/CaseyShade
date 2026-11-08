use eframe::egui;
use egui_plot::{Line, Plot, PlotPoints};

use std::sync::mpsc::Receiver;
use std::time::Duration;

use crate::data;
use crate::logging;

/// How many readings each graph keeps in memory.
const MAX_POINTS: usize = 300;

// The four graphs the user can choose from.
#[derive(PartialEq, Clone, Copy)]
enum GraphType {
    Temperature,
    Humidity,
    Pressure,
    Sunlight,
}

// One collection of sensor readings.
pub struct SensorReading {
    pub time: f64,
    pub temperature: f64,
    pub humidity: f64,
    pub pressure: f64,
    pub sunlight: f64,
}

// Stores the graph information.
pub struct GraphApp {
    receiver: Receiver<SensorReading>,

    selected_graph: GraphType,

    temperature_data: Vec<[f64; 2]>,
    humidity_data: Vec<[f64; 2]>,
    pressure_data: Vec<[f64; 2]>,
    sunlight_data: Vec<[f64; 2]>,
}

impl GraphApp {
    /// Starts with the readings already saved in the CSV file.
    pub fn new(receiver: Receiver<SensorReading>, history: &[data::StoredSample]) -> Self {
        let mut app = Self {
            receiver,

            selected_graph: GraphType::Temperature,

            temperature_data: Vec::new(),
            humidity_data: Vec::new(),
            pressure_data: Vec::new(),
            sunlight_data: Vec::new(),
        };

        for row in history {
            app.push(&SensorReading {
                time: row.hour,
                temperature: row.sample.temperature.map(f64::from).unwrap_or(f64::NAN),
                humidity: row.sample.humidity.map(f64::from).unwrap_or(f64::NAN),
                pressure: row.sample.pressure.map(f64::from).unwrap_or(f64::NAN),
                sunlight: row
                    .sample
                    .visible_light
                    .map(f64::from)
                    .unwrap_or(f64::NAN),
            });
        }

        app
    }

    fn push(&mut self, reading: &SensorReading) {
        self.temperature_data
            .push([reading.time, reading.temperature]);

        self.humidity_data.push([reading.time, reading.humidity]);

        self.pressure_data.push([reading.time, reading.pressure]);

        self.sunlight_data.push([reading.time, reading.sunlight]);

        keep_latest(&mut self.temperature_data);
        keep_latest(&mut self.humidity_data);
        keep_latest(&mut self.pressure_data);
        keep_latest(&mut self.sunlight_data);
    }
}

// Removes old readings so only the latest `MAX_POINTS` remain.
fn keep_latest(data: &mut Vec<[f64; 2]>) {
    if data.len() > MAX_POINTS {
        let extra_readings = data.len() - MAX_POINTS;
        data.drain(0..extra_readings);
    }
}

impl eframe::App for GraphApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Receive new sensor readings.
        while let Ok(reading) = self.receiver.try_recv() {
            self.push(&reading);
        }

        ui.heading("Live Sensor Dashboard");

        ui.label("Choose which graph you want to view:");

        // Dropdown menu.
        egui::ComboBox::from_label("Select graph")
            .selected_text(match self.selected_graph {
                GraphType::Temperature => "Temperature",
                GraphType::Humidity => "Humidity",
                GraphType::Pressure => "Pressure",
                GraphType::Sunlight => "Sunlight",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    &mut self.selected_graph,
                    GraphType::Temperature,
                    "Temperature",
                );

                ui.selectable_value(&mut self.selected_graph, GraphType::Humidity, "Humidity");

                ui.selectable_value(&mut self.selected_graph, GraphType::Pressure, "Pressure");

                ui.selectable_value(&mut self.selected_graph, GraphType::Sunlight, "Sunlight");
            });

        ui.separator();

        // Decide which data to display.
        let (title, y_label, data) = match self.selected_graph {
            GraphType::Temperature => (
                "Temperature Over Time",
                "Temperature (°C)",
                &self.temperature_data,
            ),

            GraphType::Humidity => ("Humidity Over Time", "Humidity (%)", &self.humidity_data),

            GraphType::Pressure => {
                ("Pressure Over Time", "Pressure (Pa)", &self.pressure_data)
            }

            GraphType::Sunlight => (
                "Sunlight Over Time",
                "Raw Light Reading",
                &self.sunlight_data,
            ),
        };

        ui.heading(title);

        if data.is_empty() {
            ui.label("No readings yet. Start saving readings or wait for the next sample.");
        }

        // Convert readings into graph points.
        let points: PlotPoints = data.clone().into();

        // Create the graph line.
        let line = Line::new(title, points);

        // Display the graph.
        Plot::new("sensor_graph")
            .view_aspect(2.0)
            .x_axis_label("Hour of day")
            .y_axis_label(y_label)
            .show(ui, |plot_ui| {
                plot_ui.line(line);
            });

        // Refresh the dashboard twice a second.
        ui.ctx().request_repaint_after(Duration::from_millis(500));
    }
}

/// Opens the graph window. Saved readings are loaded first, then new readings
/// are collected every two seconds while the window is open.
pub fn open_graph() {
    let (sender, receiver) = std::sync::mpsc::channel();

    let history = match data::read_samples(&data::csv_path()) {
        Ok(rows) => rows,
        Err(error) => {
            println!("Could not read the saved readings: {error}");
            Vec::new()
        }
    };

    let mut stream = match logging::start_stream(Duration::from_secs(2), false, Some(sender)) {
        Ok(stream) => Some(stream),
        Err(error) => {
            println!("Live readings are unavailable ({error}); showing saved data only.");
            None
        }
    };

    let graph_app = GraphApp::new(receiver, &history);

    let options = eframe::NativeOptions::default();

    let _ = eframe::run_native(
        "CaseyShade Graphs",
        options,
        Box::new(move |_cc| Ok(Box::new(graph_app))),
    );

    // Stop collecting once the window has been closed.
    if let Some(stream) = stream.as_mut() {
        stream.stop();
    }
}
