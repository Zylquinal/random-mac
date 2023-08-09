# random-mac

Random MAC address generator for Linux.

## Why you create this tool?

MAC randomization is a technique used to prevent tracking of devices by using random MAC addresses. This is a common 
technique used by people who want to protect their privacy. This tool is a simple way to generate random MAC addresses,
but as we all know, it is not a perfect solution.

Using purely random MAC addresses could make you more unique and therefore more trackable. This tool uses a random
MAC address generator that is based on real MAC addresses, based on your real network interface vendor. This way, the
generated MAC addresses are more likely to be valid and therefore less unique.

## How to use (Example)

```text
Usage: randommac [OPTIONS] <COMMAND>

Commands:
  update  Update the database
  random  Generates a random MAC address
  help    Print this message or the help of the given subcommand(s)

Options:
      --datasource <FILE>  Path to the datasource file
      --database <FILE>    Path to the database file
  -h, --help               Print help
```

### Update interfaces from specified vendor

```shell
$> sudo random-mac random vendor 'Intel Corporate' wlan0 eth1 wlan1
$> sudo random-mac random vendor 'Intel Corp' wlan0
```

### Update MAC from specified interface

```shell
sudo randommac random interface --change wlan0 eth1 wlan1
````

## Where the data stored?

The app saves the data at `$XDG_DATA_HOME` or `$HOME/.local/share`.

## As a Service

```shell
vi /etc/systemd/system/random-mac.service
```

```
[Unit]
Description=Random MAC Address Service
After=network.target

[Service]
Type=simple
ExecStart=/usr/bin/randommac random interface --change <interface> ... # replace this

[Install]
WantedBy=multi-user.target
```