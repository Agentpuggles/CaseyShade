pub struct DailyStats {
    pub min_temperature: f64,
    pub max_temperature: f64,

    pub min_humidity: f64,
    pub max_humidity: f64,

    pub min_pressure: f64,
    pub max_pressure: f64,

    pub min_sunlight: f64,
    pub max_sunlight: f64,
}
impl DailyStats {


    pub fn new() -> Self {
        Self {
            min_temperature: f64::MAX,
            max_temperature: f64::MIN,

            min_humidity: f64::MAX,
            max_humidity: f64::MIN,

            min_pressure: f64::MAX,
            max_pressure: f64::MIN,

            min_sunlight: f64::MAX,
            max_sunlight: f64::MIN,
        }
    }
    pub fn update(&mut self, reading: &crate::graph::SensorReading) {

        self.min_temperature =
            self.min_temperature.min(reading.temperature);
        self.max_temperature =
            self.max_temperature.max(reading.temperature);

        self.min_humidity =
            self.min_humidity.min(reading.humidity);
        self.max_humidity =
            self.max_humidity.max(reading.humidity);

        self.min_pressure =
            self.min_pressure.min(reading.pressure);
        self.max_pressure =
            self.max_pressure.max(reading.pressure);

        self.min_sunlight =
            self.min_sunlight.min(reading.sunlight);
        self.max_sunlight =
            self.max_sunlight.max(reading.sunlight);
    }
}