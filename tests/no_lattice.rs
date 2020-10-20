use crate::common::{await_actor_count, await_provider_count};
use crate::generated::http::{deserialize, serialize, Request, Response};
use provider_archive::ProviderArchive;
use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::{Read, Write};
use std::time::Duration;
use wascc_host::{Actor, HostBuilder, NativeCapability};

pub async fn start_and_execute_echo() -> Result<(), Box<dyn Error + Sync + Send>> {
    let h = HostBuilder::new().build();
    h.start(None).await?;
    let echo = Actor::from_file("./tests/modules/echo.wasm")?;
    let actor_id = echo.public_key();
    h.start_actor(echo).await?;
    await_actor_count(&h, 1, Duration::from_millis(50), 3).await?;

    let request = Request {
        method: "GET".to_string(),
        path: "/foo/bar".to_string(),
        query_string: "test=kthxbye".to_string(),
        header: Default::default(),
        body: b"This is a test. Do not be alarmed".to_vec(),
    };
    let buf = serialize(&request)?;
    println!("{}", buf.len());
    let res = h.call_actor(&actor_id, "HandleRequest", &buf).await?;
    println!("{}", res.len());
    let resp: Response = deserialize(&res)?;
    assert_eq!(resp.status_code, 200);
    assert_eq!(resp.status, "OK");
    let v: serde_json::Value = serde_json::from_slice(&resp.body)?;
    assert_eq!("test=kthxbye", v["query_string"].as_str().unwrap());
    Ok(())
}

pub async fn kvcounter_basic() -> Result<(), Box<dyn Error + Sync + Send>> {
    let h = HostBuilder::new().build();
    h.start(None).await?;

    let kvcounter = Actor::from_file("./tests/modules/kvcounter.wasm")?;
    let kvcounter_key = kvcounter.public_key();
    h.start_actor(kvcounter).await?;
    await_actor_count(&h, 1, Duration::from_millis(50), 3).await?;

    let mut f = File::open("./tests/modules/redis.par")?;
    let mut buf = Vec::new();
    f.read_to_end(&mut buf)?;
    let arc = ProviderArchive::try_load(&buf).unwrap();

    let redis = NativeCapability::from_archive(&arc, None)?;
    let redis_id = arc.claims().unwrap().subject;

    let mut values: HashMap<String, String> = HashMap::new();
    values.insert(
        "REDIS_URL".to_string(),
        "redis://127.0.0.1:6379".to_string(),
    );
    h.start_native_capability(redis).await?;
    // need to wait for 2 providers because extras is always there
    await_provider_count(&h, 2, Duration::from_millis(50), 3).await?;

    h.set_binding(&kvcounter_key, "wascc:keyvalue", None, redis_id, values)
        .await?;

    Ok(())
}
