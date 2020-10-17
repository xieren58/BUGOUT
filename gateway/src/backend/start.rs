use crate::backend::commands::BackendCommands;
use crate::redis_io;
use crossbeam_channel::{unbounded, Receiver, Sender};
use redis_io::RedisPool;
use std::sync::Arc;
use std::thread;

use super::*;

pub fn start_all(opts: BackendInitOptions) {
    let (kafka_commands_in, _): (Sender<BackendCommands>, Receiver<BackendCommands>) = unbounded();

    let (redis_commands_in, redis_commands_out): (
        Sender<BackendCommands>,
        Receiver<BackendCommands>,
    ) = unbounded();

    let pool_c = opts.redis_pool.clone();
    thread::spawn(move || {
        redis_io::start(
            redis_commands_out,
            &redis_io::xadd::RedisXAddCommands::create(pool_c),
        )
    });

    let bei = opts.backend_events_in.clone();
    let pool_d = opts.redis_pool.clone();
    thread::spawn(move || {
        redis_io::stream::process(
            bei,
            redis_io::stream::StreamOpts {
                entry_id_repo: redis_io::entry_id_repo::RedisEntryIdRepo::create_boxed(
                    pool_d.clone(),
                ),
                xreader: Box::new(redis_io::xread::RedisXReader { pool: pool_d }),
            },
        )
    });

    let soc = opts.session_commands_out;

    double_commands(super::doubler::DoublerOpts {
        session_commands_out: soc,
        kafka_commands_in,
        redis_commands_in,
    })
}

pub struct BackendInitOptions {
    pub backend_events_in: Sender<BackendEvents>,
    pub shutdown_in: Sender<KafkaShutdownEvent>,
    pub kafka_activity_in: Sender<KafkaActivityObserved>,
    pub session_commands_out: Receiver<BackendCommands>,
    pub redis_pool: Arc<RedisPool>,
}
