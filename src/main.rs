use structopt::StructOpt;

fn main() -> anyhow::Result<()> {
    let cli_args = rss_actions::cli::RSSActionsArgs::from_args();

    let cfg_dir = cli_args.get_cfg_dir();
    let cfg = rss_actions::Config::open(cfg_dir)?;

    let cmd = cli_args.to_cmd()?;
    let output = cmd.execute_console(&cfg)?;

    for line in output {
        println!("{}", line);
    }

    Ok(())
}
