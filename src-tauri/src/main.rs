// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // `Ensombl --mcp --api-file <api.json>` is the MCP bridge the lead agent
    // talks to, so the app needs no separate binary for it.
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("--mcp") {
        std::process::exit(ensombl_mcp::run(&args[1..]));
    }
    crm_lib::run()
}
