///////////////////////////////////////////////////////////////////////////////
// NAME:            main.rs
//
// AUTHOR:          Ethan D. Twardy <ethan.twardy@gmail.com>
//
// DESCRIPTION:     Entrypoint for the application
//
// CREATED:         12/27/2022
//
// LAST EDITED:	    12/28/2022
//
//////

use core::time::Duration;
use dbus::arg::Variant;
use dbus::message::MatchRule;
use dbus::nonblock::{Proxy, SyncConnection};
use dbus::Message;
use dbus_tokio::connection;
use futures_channel::mpsc::UnboundedReceiver;
use futures_util::stream::StreamExt;
use reqwest;
use std::collections::HashMap;
use std::sync::Arc;

struct ZoneClient<'a> {
    proxy: Proxy<'a, Arc<SyncConnection>>,
}

impl ZoneClient<'_> {
    pub fn new(connection: Arc<SyncConnection>) -> Self {
        Self {
            proxy: Proxy::new(
                "org.freedesktop.timedate1",
                "/org/freedesktop/timedate1",
                Duration::from_secs(2),
                connection,
            ),
        }
    }

    pub async fn update_timezone(&self) -> Result<(), anyhow::Error> {
        // Obtain timezone based on IP address, using open Geo-IP service.
        let timezone = reqwest::get("https://ipapi.co/timezone")
            .await?
            .text()
            .await?;
        println!("Setting timezone to {}", timezone);

        // Then, call SetTimezone method of interface org.freedesktop.timedate1
        // of object /org/freedesktop/timedate1 on service
        // org.freedesktop.timedate1
        self.proxy
            .method_call(
                "org.freedesktop.timedate1",
                "SetTimezone",
                (timezone, false),
            )
            .await?;
        Ok(())
    }
}

#[tokio::main]
pub async fn main() -> Result<(), anyhow::Error> {
    let (resource, system_bus) = connection::new_system_sync()?;

    // The resource is a task that should be spawned onto a tokio compatible
    // reactor ASAP. If the resource ever finishes, you lost connection to
    // D-Bus.
    //
    // To shut down the connection, both call _handle.abort() and drop the
    // connection.
    let _context = tokio::spawn(async {
        let error = resource.await;
        panic!("Lost connection to D-Bus: {}", error);
    });

    // Listen for changes on interface "net.connman.iwd.Station", property
    // State (to "connected")
    let rule = MatchRule::new_signal(
        "org.freedesktop.DBus.Properties",
        "PropertiesChanged",
    )
    .with_sender("net.connman.iwd");
    let (signal, mut stream): (_, UnboundedReceiver<(Message, (String,))>) =
        system_bus.add_match(rule).await?.stream();

    let client = ZoneClient::new(system_bus.clone());

    while let Some((signal, (_interface,))) = stream.next().await {
        let (interface, changed): (String, HashMap<String, Variant<String>>) =
            signal.read2()?;
        if "net.connman.iwd.Station" != interface {
            continue;
        }

        let property =
            changed.iter().find(|(name, _)| "State" == name.as_str());
        if let Some((_, state)) = property {
            if "connected" == state.0 {
                client.update_timezone().await?;
            }
        }
    }

    system_bus.remove_match(signal.token()).await?;
    unreachable!()
}

///////////////////////////////////////////////////////////////////////////////
