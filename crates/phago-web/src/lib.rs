//! # Phago Web Dashboard
//!
//! Real-time visualization and interaction for Phago colonies.
//!
//! ## Quick Start
//!
//! ```bash
//! # Start the web server
//! cargo run -p phago-web -- --port 3000
//!
//! # Open http://localhost:3000 in your browser
//! ```
//!
//! ## API Endpoints
//!
//! | Method | Path | Description |
//! |--------|------|-------------|
//! | GET | `/api/stats` | Colony statistics |
//! | GET | `/api/nodes` | All graph nodes |
//! | GET | `/api/edges` | All graph edges |
//! | GET | `/api/agents` | Active agents |
//! | GET | `/api/snapshot` | Full colony snapshot |
//! | POST | `/api/query` | Hybrid query |
//! | POST | `/api/ingest` | Ingest document |
//! | POST | `/api/tick` | Run simulation tick(s) |
//! | POST | `/api/run` | Run N ticks |
//! | WS | `/ws/events` | Real-time event stream |

pub mod routes;
pub mod state;

pub use state::AppState;
