
#[tokio::main]
async fn main() -> anyhow::Result<()>{


    println!("my_key: {}", fetch_an_integer().unwrap());

    anyhow::Ok(())
}

fn fetch_an_integer() -> redis::RedisResult<isize> {
    use redis::Commands;

    // connect to redis
    let client = redis::Client::open("redis://127.0.0.1/")?;
    let mut con = client.get_connection()?;
    // throw away the result, just make sure it does not fail
    let _: () = con.set("my_key", 42)?;
    // read back the key and return it.  Because the return value
    // from the function is a result for integer this will automatically
    // convert into one.
    con.get("my_key")
}
