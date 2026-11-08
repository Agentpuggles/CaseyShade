# CaseyShade

Our UV, Temperature, Light and humidity device

CaseyShade runs on a Raspberry Pi with a BME280 (temperature, humidity, pressure)
and a Silicon Labs Si1151 (visible light) sensor. It saves timestamped readings to
a CSV file, graphs them live, and can send the saved file to a computer over Wi-Fi.

The Si1151 has no UV-sensitive channel, so the device cannot measure UV.
`DESIGN_FEATURES.md` explains why and records what was implemented instead.
`PROJECT_WRITEUP.md` has the development story, the technical overview and the
Rust-versus-other-languages discussion.

## Running

```bash
cargo run
```

Menu entries 1–4 read the sensors and show the graphs. Option 5 starts saving a
reading to `sensor_data.csv` on a timer. Options 6 and 7 show today's statistics
and the saved rows, 8 checks the sensors, 9 explains UV, and 10 starts the web
dashboard.

## Testing without the hardware

```bash
CASEYSHADE_FAKE_SENSORS=1 cargo run
```

## Web dashboard

The Pi serves the dashboard itself, so the computer joins the CaseyShade Wi-Fi
and opens the address the Pi prints. Menu option 10 starts and stops it.

| Address | What it shows |
| --- | --- |
| `/` | Latest readings and today's statistics, refreshed every 5 s |
| `/data.csv` | Every saved reading as a CSV download |
| `/api/readings` | The same data as JSON |

The dashboard listens on all interfaces, so it is reachable from the CaseyShade
hotspot and from the normal network. It has no login, so anyone on the hotspot
can read the data.

## Settings

| Variable | Default | Meaning |
| --- | --- | --- |
| `CASEYSHADE_FAKE_SENSORS` | unset | `1` uses simulated readings |
| `CASEYSHADE_INTERVAL_SECS` | `60` | Seconds between saved readings |
| `CASEYSHADE_CSV` | `sensor_data.csv` | Where readings are saved |
| `CASEYSHADE_PORT` | `8080` | Port the web dashboard listens on |
