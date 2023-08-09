use std::fs;
use std::path::Path;
use std::process::Command;
use std::string::ToString;
use directories::{BaseDirs};
use rand::Rng;
use crate::macaddress::{DataSource, MacInformation};

mod macaddress;

struct AddressDatabase {
    path: String,
    information: Vec<Box<dyn MacInformation>>
}

impl AddressDatabase {

    fn new(path: String, information: Vec<Box<dyn MacInformation>>) -> Self {
        Self {
            path,
            information
        }
    }

    fn lookup(&self, mac: &str) -> Option<&Box<dyn MacInformation>> {
        for info in &self.information {
            if mac.starts_with(info.prefix().as_str()) {
                return Some(info);
            }
        }
        return None;
    }

    fn lookup_vendor(&self, vendor: &str) -> Option<&Box<dyn MacInformation>> {
        let vendor = vendor.to_lowercase();
        for info in &self.information {
            if info.vendor().to_lowercase().contains(&vendor) {
                return Some(info);
            }
        }
        return None;
    }

    fn save(&self) -> Result<(), String> {
        let serialize = match serde_json::to_string(&self.information) {
            Ok(json) => json,
            Err(_) => return Err(String::from("Failed to serialize JSON"))
        };

        return match fs::write(&self.path, serialize) {
            Ok(_) => Ok(()),
            Err(_) => Err(String::from("Failed to write JSON"))
        };
    }

    fn random_from_prefix(prefix: &str) -> String {
        let mut rng = rand::thread_rng();
        let mut mac = String::from(prefix);
        for _ in 0..3 {
            mac.push_str(format!(":{:02X}", rng.gen_range(0..255)).as_str());
        }
        return mac;
    }

}

fn main() {

    let cli = build_cli().get_matches();

    let app_dir = app_dir();
    if !Path::new(&app_dir).exists(){
        fs::create_dir_all(app_dir).expect("Failed to create app directory");
    }

    let datasource = match  cli.get_one::<String>("datasource") {
        Some(datasource) => datasource.to_string(),
        None => datasource()
    };

    let database = match cli.get_one::<String>("database") {
        Some(database) => database.to_string(),
        None => database()
    };

    match cli.subcommand() {
        Some(("update", _)) => {
            update(datasource.clone(), database.clone()).unwrap();
            return;
        },
        Some(("random", sub_matches)) => {
            match sub_matches.subcommand() {
                Some(("prefix", sub_matches)) => {
                    let prefix = match sub_matches.get_one::<String>("prefix") {
                        Some(prefix) => prefix,
                        None => {
                            println!("No prefix given!");
                            return;
                        }
                    };

                    let database = match setup_data(datasource.clone(), database.clone()) {
                        Ok(database) => database,
                        Err(error) => {
                            println!("{}", error);
                            return;
                        }
                    };

                    let interfaces = sub_matches.get_many::<String>("interface")
                        .unwrap_or_default().map(|v| v.to_string()).collect::<Vec<_>>();

                    match macaddress::verify_prefix(&prefix) {
                        Ok(_) => (),
                        Err(err) => {
                            println!("{}", err);
                            return;
                        }
                    }

                    if interfaces.is_empty() {
                        println!("Generating random MAC address with prefix {}...", prefix);
                        println!("Random MAC address: {}", AddressDatabase::random_from_prefix(&prefix));
                        return;
                    }

                    if !is_root() {
                        println!("You need to be root to run this command!");
                        return;
                    }

                    let mac = match database.lookup(prefix) {
                        Some(information) => information,
                        None => {
                            println!("No vendor found with prefix {}!", prefix);
                            return;
                        }
                    };

                    for interface in &interfaces {
                        update_mac_by_info(mac, interface);
                    }

                },
                Some(("vendor", sub_matches)) => {
                    let vendor = match sub_matches.get_one::<String>("vendor") {
                        Some(vendor) => vendor,
                        None => {
                            println!("No vendor given!");
                            return;
                        }
                    };

                    let database = match setup_data(datasource.clone(), database.clone()) {
                        Ok(database) => database,
                        Err(error) => {
                            println!("{}", error);
                            return;
                        }
                    };

                    let interfaces = sub_matches.get_many::<String>("interface")
                        .unwrap_or_default().map(|v| v.to_string()).collect::<Vec<_>>();

                    let mac = match database.lookup_vendor(vendor) {
                        Some(information) => information,
                        None => {
                            println!("No vendor found with name {}!", vendor);
                            return;
                        }
                    };

                    if interfaces.is_empty() {
                        let random_mac = mac.random_from_prefix();
                        println!("Random MAC address: {}", random_mac);
                        return;
                    }

                    if !is_root() {
                        println!("You need to be root to run this command!");
                        return;
                    }

                    println!("Generating random MAC address with vendor {}...", mac.vendor());
                    for interface in &interfaces {
                        update_mac_by_info(mac, interface);
                    }

                },
                Some(("interface", sub_matches)) => {
                    let interfaces = sub_matches.get_many::<String>("interface")
                        .unwrap_or_default().map(|v| v.to_string()).collect::<Vec<_>>();

                    let change = match sub_matches.get_one::<bool>("change") {
                        Some(change) => change,
                        None => {
                            println!("No change given!");
                            return;
                        }
                    };

                    let database = match setup_data(datasource.clone(), database.clone()) {
                        Ok(database) => database,
                        Err(error) => {
                            println!("{}", error);
                            return;
                        }
                    };

                    if interfaces.is_empty() {
                        println!("No interfaces given!");
                        return;
                    }

                    if !is_root() && *change {
                        println!("You need to be root to run this command!");
                        return;
                    }

                    random_interface(&database, interfaces, *change)

                },
                _ => unreachable!("This should not happen!")
            }
        },
        _ => unreachable!("This should not happen!")
    }

}

fn build_cli() -> clap::Command {
    clap::Command::new("random-mac")
        .bin_name("random-mac")
        .subcommand_required(true)
        .subcommand(
            clap::command!("update")
                .about("Update the database")
        )
        .subcommand(
            clap::command!("random")
                .about("Generates a random MAC address")
                .subcommand_required(true)
                .subcommand(
                    clap::command!("vendor")
                        .about("Generates a random MAC address from a vendor")
                        .arg(
                            clap::arg!(<vendor> "Vendor to use")
                                .required(true)
                                .index(1)
                        )
                        .arg(
                            clap::arg!([interface] ... "Change the MAC address for interface")
                                .required(false)
                                .trailing_var_arg(true)
                                .index(2)
                        )
                )
                .subcommand(
                    clap::command!("prefix")
                        .about("Generates a random MAC address from a prefix")
                        .arg(
                            clap::arg!(<prefix> "MAC address prefix to use")
                                .required(true)
                                .index(1)
                        )
                        .arg(
                            clap::arg!([interface] ... "Change the MAC address for interface")
                                .required(false)
                                .trailing_var_arg(true)
                                .index(2)
                        )
                )
                .subcommand(
                    clap::command!("interface")
                        .about("Generates a random MAC address for the given interfaces")
                        .arg(
                            clap::arg!(-c --change "Change the MAC address")
                                .required(false)
                        )
                        .arg(
                            clap::arg!(<interface> ... "Interfaces to use")
                                .required(true)
                                .trailing_var_arg(true)
                        )
                )
        )
        .arg(
            clap::arg!(--datasource <FILE> "Path to the datasource file")
                .required(false)
        )
        .arg(
            clap::arg!(--database <FILE> "Path to the database file")
                .required(false)
        )
}

fn update(datasource: String, database: String) -> Result<(), String> {
    println!("Updating database...");

    let datasource = setup_datasource(&datasource);
    let information = fetch(datasource, &database, false)?;

    let addr_database = AddressDatabase::new(database, information);
    addr_database.save()?;

    println!("Database updated, found {} entries!", addr_database.information.len());

    return Ok(());
}

fn random_interface(database: &AddressDatabase, interface: Vec<String>, update: bool) {
    println!("Generating random MAC address for interface {}...", interface.join(", "));
    for interface in interface {
        let mac = match mac_address::mac_address_by_name(&interface) {
            Ok(mac) => mac,
            Err(err) => {
                println!("Failed to get MAC address for interface {}: {}", interface, err);
                continue;
            }
        };

        let mac = match mac {
            Some(mac) => mac,
            None => {
                println!("No MAC address found for interface {}!", interface);
                continue;
            }
        }.to_string();

        match database.lookup(&mac) {
            Some(result) => {
                let new_mac = result.random_from_prefix();
                if update {
                    match update_mac(&interface, &new_mac) {
                        Ok(_) => println!("MAC address for interface {} changed to {}", interface, new_mac),
                        Err(err) => println!("Failed to change MAC address for interface {}: {}", interface, err)
                    }
                } else {
                    println!("MAC address for interface {}: {}", interface, new_mac)
                }
            },
            None => println!("No registered vendor found for interface {}!", interface)
        }
    }
}

fn update_mac_by_info(mac: &Box<dyn MacInformation>, interface: &str) {
    let random_mac = mac.random_from_prefix();
    match mac_address::mac_address_by_name(interface) {
        Ok(mac) => {
            if mac.is_none() {
                println!("Interface '{}' doesn't exist, skipping...", interface);
                return;
            }
        },
        Err(_) => {
            println!("Failed to get MAC address of {}, skipping!", interface);
            return;
        }
    }
    match update_mac(&interface, &random_mac) {
        Ok(_) => println!("Updated MAC address of {} to {}", interface, random_mac),
        Err(error) => println!("Failed to update MAC address of {}: {}", interface, error)
    }
}

fn update_mac(interface: &str, mac: &str) -> Result<(), String> {
    let turn_off = Command::new("ip")
        .arg("link")
        .arg("set")
        .arg("dev")
        .arg(interface)
        .arg("down")
        .output();

    match turn_off {
        Ok(_) => (),
        Err(_) => return Err(format!("Failed to turn off interface {}!", interface))
    }

    let change = Command::new("ip")
        .arg("link")
        .arg("set")
        .arg("dev")
        .arg(interface)
        .arg("address")
        .arg(mac)
        .output();

    match change {
        Ok(_) => (),
        Err(_) => return Err(format!("Failed to change MAC address for interface {}!", interface))
    }

    let turn_on = Command::new("ip")
        .arg("link")
        .arg("set")
        .arg("dev")
        .arg(interface)
        .arg("up")
        .output();

    match turn_on {
        Ok(_) => Ok(()),
        Err(_) => return Err(format!("Failed to turn on interface {}!", interface))
    }

}

fn setup_data(datasource: String, database: String) -> Result<AddressDatabase, String> {
    let datasource = setup_datasource(&datasource);

    return if Path::new(&database).exists() {
        let content = fs::read_to_string(&database)
            .expect(&*format!("Failed to read {:?}!", database));

        match macaddress::convert(datasource.name, content) {
            Ok(result) => Ok(AddressDatabase::new(database, result)),
            Err(_) => return Err(String::from("Failed to parse JSON"))
        }
    } else {
        println!("Database not found, downloading...");
        let information = fetch(datasource, &database, true)?;
        let addr_database = AddressDatabase::new(database, information);
        addr_database.save()?;
        println!("Database downloaded, found {} entries!", addr_database.information.len());
        Ok(addr_database)
    }
}

fn setup_datasource(path: &String) -> DataSource {
    if !Path::new(path).exists() {
        let datasource = DataSource {
            url: String::from("https://maclookup.app/downloads/json-database/get-db"),
            name: String::from("maclookupapp")
        };
        let serialize = serde_json::to_string(&datasource)
            .expect("Failed to serialize default datasource!");

        fs::write(path, serialize)
            .expect(&*format!("Failed to write default datasource: {:?}", path));

        return datasource;
    }

    DataSource::from_file(path.as_ref())
        .expect("Failed to read datasource!")
}

fn fetch(datasource: DataSource, database: &String, write: bool) -> Result<Vec<Box<dyn MacInformation>>, String> {
    let information = datasource.fetch_information()?;
    if write {
        let serialize = match serde_json::to_string(&information) {
            Ok(json) => json,
            Err(_) => return Err(String::from("Failed to serialize JSON"))
        };

        return match fs::write(database, serialize) {
            Ok(_) => Ok(information),
            Err(_) => Err(String::from("Failed to write JSON"))
        };
    }
    return Ok(information);
}

#[inline]
fn is_root() -> bool {
    let uid = unsafe { libc::getuid() };
    uid == 0
}

#[inline]
fn datasource() -> String {
    return format!("{}/{}", app_dir(), "datasource.json");
}

#[inline]
fn database() -> String {
    return format!("{}/{}", app_dir(), "database.json");
}

#[inline]
fn app_dir() -> String {
    let user = BaseDirs::new().unwrap();
    return format!("{}/{}", user.data_dir().to_str().unwrap(), "random-mac");
}
