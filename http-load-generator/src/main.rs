use clap::{command, value_parser, Arg, ArgAction};
use futures::future;
use std::process;
use tokio::sync::mpsc;

mod consume;
mod experiment;
mod generator;
mod metric;
mod receiver;
mod request;

use crate::consume::{Consume, ConsumeConfiguration};
use crate::receiver::{ExperimentReceiver, ExperimentReceiverConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ctrlc::set_handler(move || {
        println!("received Ctrl+C!");
        process::exit(0);
    })
    .expect("Error setting Ctrl-C handler");

    let mut matches = command!() // requires `cargo` feature
        .next_line_help(true)
        .arg(Arg::new("secret-key")
            .required(false)
            .long("secret-key")
            .action(ArgAction::Set)
            .default_value("QJUHsPhnA0eiqHuJqsPgzhDozYO4f1zh")
            .help("<key> is a 32 character string that must match the key being passed to the notifications-service")
        )
        .arg(Arg::new("broker-list")
            .required(true)
            .action(ArgAction::Set)
            .short('b')
            .long("brokers")
            .help("<broker-list> is a comma-seperated list of brokers. E.g.  For a single local broker `localhost:9092`. For multiple brokers `localhost:9092,localhost:9093`")
        )
        .arg(Arg::new("topic")
            .required(true)
            .long("topic")
            .default_value("experiment")
            .action(ArgAction::Set)
        )
        .arg(Arg::new("group-id")
            .required(true)
            .long("group-id")
            .action(ArgAction::Set)
        )
        .arg(Arg::new("consumer-wait-before-send")
            .required(false)
            .long("consumer-wait-before-send")
            .action(ArgAction::Set)
            .default_value("60")
            .value_parser(value_parser!(u8))
            .help("Time the consumer should wait before forwarding the experiment to the receiver")
        )
        .arg(Arg::new("hosts-file")
            .required(true)
            .long("hosts-file")
            .action(ArgAction::Set)
            .help("The file containing the list of hosts to be queried")
        )
        .arg(Arg::new("requestor-lag")
            .required(false)
            .long("requestor-lag")
            .action(ArgAction::Set)
            .default_value("5")
            .value_parser(value_parser!(u8))
            .help("Time the requestor lags behind the generator.")
        )
        .arg(Arg::new("requestor-retries")
            .required(false)
            .long("requestor-retries")
            .action(ArgAction::Set)
            .default_value("2")
            .value_parser(value_parser!(u8))
            .help("The number of retries in case a request fails due to a server error.")
        )
        .arg(Arg::new("requestor-max-in-flight")
            .required(false)
            .long("requestor-max-in-flight")
            .action(ArgAction::Set)
            .default_value("50")
            .value_parser(value_parser!(u16))
            .help("The maximum number of connections to a host.")
        )
        .arg(Arg::new("min-batch-size")
            .required(false)
            .long("min-batch-size")
            .action(ArgAction::Set)
            .default_value("100")
            .value_parser(value_parser!(u16))
            .help("The minimum number of queries that has to be performed per second to each host.")
        )
        .arg(Arg::new("max-batch-size")
            .required(false)
            .long("max-batch-size")
            .action(ArgAction::Set)
            .default_value("200")
            .value_parser(value_parser!(u16))
            .help("The maximum number of queries that can be performed per second to each host.")
        )
        .arg(Arg::new("stable-rate-duration")
            .required(false)
            .long("stable-rate-duration")
            .action(ArgAction::Set)
            .default_value("60")
            .value_parser(value_parser!(u16))
            .help("The number of seconds during which the rate at which the queries are performed to each host remains stable.")
        )
        .get_matches();

    let consume_config = ConsumeConfiguration::from(&mut matches);
    let consume = Consume::new(consume_config);

    let mut handles = vec![];
    let (experiment_tx, experiment_rx) = mpsc::channel(1000);
    let receiver_config = ExperimentReceiverConfig::from(&mut matches);
    let receiver = ExperimentReceiver::new(receiver_config, experiment_rx);
    handles.push(tokio::spawn(receiver.start()));
    handles.push(tokio::spawn(async move {
        consume.start(experiment_tx).await;
    }));
    future::join_all(handles).await;
    Ok(())
}
