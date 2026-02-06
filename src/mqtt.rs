//! MQTT client for receiving chyron messages
//!
//! Connects to an MQTT broker and subscribes to a topic.
//! Messages received are forwarded to the main loop for display.

use rumqttc::{Client, Event, MqttOptions, Packet, QoS};
use serde::Deserialize;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;

const DEFAULT_HOST: &str = "192.168.23.123";
const DEFAULT_PORT: u16 = 1883;
const DEFAULT_TOPIC: &str = "wallfacer";
const DEFAULT_TTL: f32 = 60.0;

/// A chyron message with text and time-to-live
#[derive(Debug, Clone)]
pub struct ChyronMessage {
    pub text: String,
    pub ttl: f32,
}

/// JSON format for incoming messages (optional)
#[derive(Deserialize)]
struct JsonMessage {
    text: String,
    #[serde(default = "default_ttl")]
    ttl: f32,
}

fn default_ttl() -> f32 {
    DEFAULT_TTL
}

/// MQTT client that receives messages in a background thread
pub struct MqttClient {
    receiver: Receiver<ChyronMessage>,
    _thread: thread::JoinHandle<()>,
}

impl MqttClient {
    /// Create a new MQTT client and connect to the broker.
    /// Fails immediately if connection cannot be established.
    pub fn new(host: &str, topic: &str) -> Result<Self, String> {
        let host = if host.is_empty() { DEFAULT_HOST } else { host };
        let topic = if topic.is_empty() { DEFAULT_TOPIC } else { topic };

        let mut options = MqttOptions::new("wallfacer", host, DEFAULT_PORT);
        options.set_keep_alive(Duration::from_secs(30));

        let (client, mut connection) = Client::new(options, 10);

        // Subscribe to topic
        client
            .subscribe(topic, QoS::AtMostOnce)
            .map_err(|e| format!("Failed to subscribe to topic '{}': {}", topic, e))?;

        // Test connection by polling once - fail fast if broker unreachable
        let first_event = connection.iter().next();
        match first_event {
            Some(Ok(_)) => {}
            Some(Err(e)) => {
                return Err(format!(
                    "Failed to connect to MQTT broker at {}:{} - {}",
                    host, DEFAULT_PORT, e
                ));
            }
            None => {
                return Err(format!(
                    "Failed to connect to MQTT broker at {}:{} - connection closed",
                    host, DEFAULT_PORT
                ));
            }
        }

        let (sender, receiver) = mpsc::channel();
        let topic_owned = topic.to_string();

        let handle = thread::spawn(move || {
            Self::message_loop(connection, sender, &topic_owned);
        });

        eprintln!("MQTT: Connected to {}:{}, subscribed to '{}'", host, DEFAULT_PORT, topic);

        Ok(Self {
            receiver,
            _thread: handle,
        })
    }

    fn message_loop(
        mut connection: rumqttc::Connection,
        sender: Sender<ChyronMessage>,
        topic: &str,
    ) {
        for event in connection.iter() {
            match event {
                Ok(Event::Incoming(Packet::Publish(publish))) => {
                    if publish.topic == topic {
                        if let Ok(text) = String::from_utf8(publish.payload.to_vec()) {
                            let text = text.trim();
                            if !text.is_empty() {
                                // Try to parse as JSON, fall back to plain text
                                let msg = if let Ok(json) = serde_json::from_str::<JsonMessage>(text) {
                                    ChyronMessage {
                                        text: json.text,
                                        ttl: json.ttl,
                                    }
                                } else {
                                    ChyronMessage {
                                        text: text.to_string(),
                                        ttl: DEFAULT_TTL,
                                    }
                                };
                                if sender.send(msg).is_err() {
                                    // Main thread gone, exit
                                    break;
                                }
                            }
                        }
                    }
                }
                Ok(_) => {}
                Err(e) => {
                    eprintln!("MQTT error: {}", e);
                    // Continue trying - connection may recover
                }
            }
        }
    }

    /// Poll for the latest message (non-blocking).
    /// Returns the most recent message if any arrived, discarding older ones.
    pub fn poll(&self) -> Option<ChyronMessage> {
        let mut latest = None;
        while let Ok(msg) = self.receiver.try_recv() {
            latest = Some(msg);
        }
        latest
    }

    /// Default MQTT host
    pub fn default_host() -> &'static str {
        DEFAULT_HOST
    }

    /// Default MQTT topic
    pub fn default_topic() -> &'static str {
        DEFAULT_TOPIC
    }
}
