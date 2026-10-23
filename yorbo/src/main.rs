mod config;
mod partitions;
mod toyboot;
mod vlnk;

use std::path::PathBuf;
use std::env::current_exe;

use anyhow::{Context, Result};
use hadris::fs::Stored::No;
use input_macro::input;
use config::Config;
use clap::{Parser, ValueEnum};

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
enum Boot{
    Ventoy,
    Iso,
}

#[derive(Parser, Debug)]
struct Args{
    #[arg(short, long)]
    boot: Boot,
    #[arg(short, long)]
    file: Option<PathBuf>,
    #[arg(short, long)]
    reboot: bool
}

fn main() -> Result<()> {
    let args = Args::parse();
    if matches!(args.boot, Boot::Iso) && args.file.is_none() {
            eprintln!("Error: The --file or -f argument is required when --boot is set to 'iso'.");
            std::process::exit(1);
        }

    if !matches!(args.boot, Boot::Iso) && args.file.is_some() {
        eprintln!("Error: The --file or -f argument is only valid when --boot is set to 'iso'.");
        std::process::exit(1);
    }

    let mut cfg_path = current_exe().context("i got alzheimers")?;
    cfg_path.pop();
    cfg_path.push("config");
    let mut cfg: Config = match confy::load_path(&cfg_path) {
        Ok(cfg) => cfg,
        _ => reconfigure()?
    };
    println!("Config loaded");
    let mut toyboot_partition = partitions::find(cfg.guid);
    if toyboot_partition.is_err() {
        cfg = reconfigure()?;
        toyboot_partition = partitions::find(cfg.guid);
    }

    let toyboot_partition = toyboot_partition.unwrap();
    if !toyboot_partition.verify_format()? {
        cfg = reconfigure()?;
    }
    println!("boot partition loaded"); 
    confy::store_path(&cfg_path, &cfg);
    let partition_letter = toyboot_partition.ensure_mounted().expect("Failed to mount partition");
    if args.boot == Boot::Iso {
        toyboot::set_iso_boot(&toyboot_partition, args.file.unwrap());
    }

    else if args.boot == Boot::Ventoy {
        toyboot::set_ventoy_boot(&toyboot_partition);
    }

    if args.reboot {
        toyboot::toyboot(&toyboot_partition)?;
    }
    Ok(())
    
}

fn reconfigure() -> Result<Config> {
    let partitions = partitions::get_partitions().expect("Failed to fetch partitions");
    println!("Failed to find toyboot partition. Please select the partition or reinstall:");
    for (i, partition) in partitions.iter().enumerate() {
        let mountpoint = match partition.mountpoint.as_ref() {
            Some(mp) => mp.to_string(),
            _ => "Unmounted".to_string()
        };
        let fsformat = match partition.partition_type.as_ref() {
            Some(fs) => fs.to_string(),
            _ => "Null".to_string()
        };
        println!("{}. {} : {} mb {} (disk {})", 
        i+1, mountpoint, partition.size / (1024 * 1024), fsformat, partition.disk_number);
    }
    let selected_partition_num: usize = input!("").parse().expect("Please enter a valid number. quitting");
    let selected_partition = partitions.get(selected_partition_num -1).expect("Please enter a valid number. quitting");
    if selected_partition.verify_format()? {
        return Ok(Config{
            guid: selected_partition.guid,
            partition: None
        });
    }
    return reconfigure();    

}