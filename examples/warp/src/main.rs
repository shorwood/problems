//! Serves the warp application on `127.0.0.1:3000`.

use problems_example_warp::app;

#[tokio::main]
async fn main() {
    warp::serve(app()).run(([127, 0, 0, 1], 3000)).await;
}
