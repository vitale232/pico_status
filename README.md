# Pico Status

This repository is a [Raspberry Pi Pico W](https://www.raspberrypi.com/products/raspberry-pi-pico/)
project. The concept is that the Rasp Pi microcontroller will act as a server
over the local Wifi network. The server is currently implemented in micropython,
but obv it's gotta be Rust one day.

The server accepts TCP requests over port 80 at four routes:

- `/green`: Lights up display as green with black text
- `/yellow`: Lights up display as yellow with black text
- `/red`: Lights up display as red with white text
- `/late`: Lights up display as 🍆eggplant🍆 with white text

When a request is successfully processed, the Pico will paint the
[Waveshare Pico LDC 1.14](https://www.waveshare.com/wiki/Pico-LCD-1.14)
with the appropriate color.

Currently, the server supports seven query params.

- `line1`: A line of text that will be displayed on the first line of the LCD
- `line2`: A line of text that will be displayed on the second line of the LCD
- `line3`: A line of text that will be displayed on the third line of the LCD
- `line4`: A line of text that will be displayed on the fourth line of the LCD
- `line5`: A line of text that will be displayed on the fifth line of the LCD
- `line6`: A line of text that will be displayed on the sixth line of the LCD
- `line7`: A line of text that will be displayed on the seventh line of the LCD

## Usage

If you're totally new to Pico W, this is a really great tutorial [to get up
and running quickly](https://projects.raspberrypi.org/en/projects/get-started-pico-w)

Create a `server_micropython/secrets.py` file with 2 variables:

1. `ssid` - The name of your WiFi network (e.g. "pepper-tush")
2. `password`- The password to the network (e.g. lolYouThought-you-had-me-2282)

Then, use the [Thonny](https://thonny.org/)
IDE to flash the microcontroller with the micropython runtime. After that,
simply save the `main.py` and `secrets.py` files to the controller. Whenever
the device is powered, you should now expect it to boot and attempt to
connect to the configured WiFi! If it's successful, the screen will be painted
pink and the IPv4 address will be painted on the LCD. You should now
be able to control the device using HTTP! Open it in your browser, or try
from the CLI.

### Example CLI Usage

Currently, I can't get this to work with curl, which seems quite odd. Whatevs.
Let's use wget. Here's a sample request to paint the screen green with a celebratory
message:

```shell
wget -O - "http://xxx.xxx.x.xxx/red?line1=                    08:00 am&line2= Busy&line3= (Busy)&line5= Meeting goes until:&line6=  08:30 am (Demo Meeting)&line7=  1 attendees"
```

![a raspberry pi pico w connected to a Pico LCD 1.14 displaying a status indicating the logged in usr is in a meeting](./assets/in_meeting.png "Raspberry Pi Web Server")

## Automated Client for Microsoft Teams/Outlook users

A client application has been created to integrate the pico w and its LCD
with MS Teams and outlook. The app requires that you configure an OAuth application
in the Azure Portal for your work or school managed account. Once that is setup,
you'll need to take note of some key variables from your configuration. For convenience,
you may want to store them in a local `.env` file, or perhaps your password
manager.

Here's an example of saving the required info to an `.env` file:

```shell
touch client/.env

# CLIENT_ID=
# CLIENT_SECRET=
# TENANT_ID=
# PI_IP=
```

### Running the Client

The client application has been developed with a command line interface,
which should provide end users with lots of flexibility in how they use
and configure the tool.

The help can be accessed once the project is installed by passing the
help flag to the tool:

```shell
pico-client --help
```

Which prints:

```text
pico-client 0.1.0
Application that updates Raspberry Pi Pico W with MS Teams/Outlook status

USAGE:
    pico-client [OPTIONS] <PICO_IP> <CLIENT_ID> [TENANT_ID]

ARGS:
    <PICO_IP>      The IP address of the Pico your connecting to (e.g. 169.420.1.469)
    <CLIENT_ID>    The OAuth Client ID of the registered application from Azure Portal
    <TENANT_ID>    The MS tenant ID to connect to, including the 'common' tennant which is
                   default [default: common]

OPTIONS:
    -a, --auth-wait-for <AUTH_WAIT_FOR>
            The time, in seconds, that the pico-status tool will wait before killing the local
            server that supports OAuth [default: 3]

    -h, --help
            Print help information

    -p, --poll-after <POLL_AFTER>
            The time, in seconds, that the tool waits before polling MS for your status and updating
            the Pico W [default: 60]

    -r, --refresh-expiry-padding <REFRESH_EXPIRY_PADDING>
            The number of seconds that the pico-client will use to 'pad', or trim, the auth token's
            expiry [default: 120]

    -s, --scope <SCOPE>
            The Scope to require on the auth token. Only scopes configured in the OAuth app will
            work [default: "Presence.Read Calendars.Read offline_access"]

    -v, --verbose
            Include exxxtra verbose tracing

    -V, --version
            Print version information
```

An example of executing the client application would be something like:

```shell
pico-client 127.0.0.2 01e89a7d-fa38-4c97-9e8a-f97d932d5fdb common --auth-wait-for 30
```

From there, the client app takes over. It will fetch your Presence and
CalendarView from the MS Graph API, interpret the results into a text based summary,
and make an HTTP requst to the Pi's IP. If the pi server is running,
it should update the LCD! There are some constant variables in the app that control
the frequency of updates, which will likely be migrated to a clap-based CLI.

