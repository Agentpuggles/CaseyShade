use eframe::egui;
use egui_plot::{Line, Plot, PlotPoints};

use std::sync::mpsc::Receiver;
use std::time::Duration;

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
    pub fn new(receiver: Receiver<SensorReading>) -> Self {
        Self {
            receiver,

            selected_graph: GraphType::Temperature,

            temperature_data: Vec::new(),
            humidity_data: Vec::new(),
            pressure_data: Vec::new(),
            sunlight_data: Vec::new(),
        }
    }
}

// Removes old readings so only the latest 100 remain.
fn keep_latest(data: &mut Vec<[f64; 2]>) {
    if data.len() > 100 {
        let extra_readings = data.len() - 100;
        data.drain(0..extra_readings);
    }
}

impl eframe::App for GraphApp {
    fn ui(
        &mut self,
        ui: &mut egui::Ui,
        _frame: &mut eframe::Frame,
    ) {
        // Receive new sensor readings.
        while let Ok(reading) = self.receiver.try_recv() {
            self.temperature_data.push([
                reading.time,
                reading.temperature,
            ]);

            self.humidity_data.push([
                reading.time,
                reading.humidity,
            ]);

            self.pressure_data.push([
                reading.time,
                reading.pressure,
            ]);

            self.sunlight_data.push([
                reading.time,
                reading.sunlight,
            ]);
        }

        // Keep only the latest 100 readings.
        keep_latest(&mut self.temperature_data);
        keep_latest(&mut self.humidity_data);
        keep_latest(&mut self.pressure_data);
        keep_latest(&mut self.sunlight_data);

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

                ui.selectable_value(
                    &mut self.selected_graph,
                    GraphType::Humidity,
                    "Humidity",
                );

                ui.selectable_value(
                    &mut self.selected_graph,
                    GraphType::Pressure,
                    "Pressure",
                );

                ui.selectable_value(
                    &mut self.selected_graph,
                    GraphType::Sunlight,
                    "Sunlight",
                );
            });

        ui.separator();

        // Decide which data to display.
        let (title, y_label, data) = match self.selected_graph {
            GraphType::Temperature => (
                "Temperature Over Time",
                "Temperature (°C)",
                &self.temperature_data,
            ),

            GraphType::Humidity => (
                "Humidity Over Time",
                "Humidity (%)",
                &self.humidity_data,
            ),

            GraphType::Pressure => (
                "Pressure Over Time",
                "Pressure (hPa)",
                &self.pressure_data,
            ),

            GraphType::Sunlight => (
                "Sunlight Over Time",
                "Raw Light Reading",
                &self.sunlight_data,
            ),
        };

        ui.heading(title);

        // Convert readings into graph points.
        let points: PlotPoints = data.clone().into();

        // Create the graph line.
        let line = Line::new(title, points);

        // Display the graph.
        Plot::new("sensor_graph")
            .view_aspect(2.0)
            .x_axis_label("Time (seconds)")
            .y_axis_label(y_label)
            .show(ui, |plot_ui| {
                plot_ui.line(line);
            });

        // Refresh the dashboard every 200 milliseconds.
        ui.ctx().request_repaint_after(
            Duration::from_millis(200)
        );
    }
}
pub fn open_graph() {
    let (sender, receiver) =
        std::sync::mpsc::channel();

    let graph_app = GraphApp::new(receiver);

    let options = eframe::NativeOptions::default();

    let _ = eframe::run_native(
        "CaseyShade Graphs",
        options,
        Box::new(move |_cc| {
            Ok(Box::new(graph_app))
        }),
    );
}