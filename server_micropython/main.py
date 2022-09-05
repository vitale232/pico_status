from machine import Pin, PWM
import socket
import network
from time import sleep

from secrets import ssid, password
from waveshare import LCD_1inch14, BL
import paint

ERROR_AFTER_SECS = 10


def unquote(string):
    """unquote('abc%20def') -> b'abc def'."""
    _hexdig = "0123456789ABCDEFabcdef"
    _hextobyte = None

    # Note: strings are encoded as UTF-8. This is only an issue if it contains
    # unescaped non-ASCII characters, which URIs should not.
    if not string:
        return b""

    if isinstance(string, str):
        string = string.encode("utf-8")

    bits = string.split(b"%")
    print(f"bits={bits}")
    if len(bits) == 1:
        return string

    res = [bits[0]]
    append = res.append

    # Delay the initialization of the table to not waste memory
    # if the function is never called
    if _hextobyte is None:
        _hextobyte = {
            (a + b).encode(): bytes([int(a + b, 16)]) for a in _hexdig for b in _hexdig
        }

    for item in bits[1:]:
        try:
            append(_hextobyte[item[:2]])
            append(item[2:])
        except KeyError:
            append(b"%")
            append(item)

    return b"".join(res)


def connect():
    # Connect to WLAN
    wlan = network.WLAN(network.STA_IF)
    wlan.active(True)
    wlan.connect(ssid, password)

    attempts = 0
    while wlan.isconnected() == False:
        print("Waiting for connection...")
        sleep(1)
        attempts += 1
        if attempts > ERROR_AFTER_SECS:
            raise RuntimeError("Could not connect to WiFi!")

    ip = wlan.ifconfig()[0]
    print(f"Connected on {ip}: {wlan.isconnected()}")
    return ip


def open_socket(ip):
    address = (ip, 80)
    connection = socket.socket()
    connection.bind(address)
    connection.listen(1)
    return connection


def parse_request(req, state):
    print(f"parsing request::{req}\nwith state::{state}")

    url = ""
    try:
        url = req.split()[1]
    except IndexError:
        print(f"INDEX ERROR!! PASSING!")
        pass
    print(f"url {url}")

    url_parts = url.split("?")
    path = url_parts[0]

    if path == "/green":
        state = "GREEN"
    if path == "/yellow":
        state = "YELLOW"
    if path == "/red":
        state = "RED"

    line1, line2, line3, line4, line5, line6, line7 = "", "", "", "", "", "", ""
    print(f"url_parts={url_parts}")
    if len(url_parts) == 1:
        pass
    else:
        (line1, line2, line3, line4, line5, line6, line7) = parse_text(url_parts[1])

    print(f"ret:: {(state, line1, line2, line3, line4, line5, line6, line7)}")
    return (state, line1, line2, line3, line4, line5, line6, line7)


def parse_text(text):
    print(f"parsing text: {text}")
    text = text.replace("?", "").split("&")
    line1, line2, line3, line4, line5, line6, line7 = "", "", "", "", "", "", ""

    for el in text:
        print(f"{el}")
        try:
            (param, val) = el.split("=")
        except Exception as exc:
            print(f"Could not unpack params due to the exception: {exc}. Skipping.")
            continue
        print(f"param={param}, val={val}")
        if param == "line1":
            line1 = unquote(val)
        if param == "line2":
            line2 = unquote(val)
        if param == "line3":
            line3 = unquote(val)
        if param == "line4":
            line4 = unquote(val)
        if param == "line5":
            line5 = unquote(val)
        if param == "line6":
            line6 = unquote(val)
        if param == "line7":
            line7 = unquote(val)
    return (line1, line2, line3, line4, line5, line6, line7)


def render(state):
    return f"""
HTTP/1.1 200 OK
Cache-Control: no-cache
Server: pi-in-the-sky
Content-Type: text/html

            <!DOCTYPE html>
            <html>
            <a href="/green">Green</a>
            <br /> <br />
            <a href="/yellow">Yellow</a>
            <br /> <br />
            <a href="/red">Red</a>
            <p>Screen is {state}</p>
            </body>
            </html>
            """


def serve(connection, lcd):
    print("Staring server...")
    color_state = "GREEN"
    while True:
        client = connection.accept()[0]
        request = client.recv(1024)
        request = str(request)
        print(f"Incoming Request:\n{request}")
        try:
            paint.paint_status(lcd, *parse_request(request, color_state))
            html = render(color_state)
            client.send(html)
            client.close()
        except Exception as exc:
            print(f"An exception occurred: {exc}")
            client.close()


def listen_for_retry_click(lcd, a_button, b_button):
    while True:
        if a_button.value() == 0:
            paint.paint_reconnect(lcd, "Trying new server...", "If this fails, reboot!")
            sleep(2)
            return
        if b_button.value() == 0:
            paint.paint_reconnect(
                lcd, "Trying to re-establish server...", "If this fails, reboot!"
            )
            sleep(2)
            return


if __name__ == "__main__":
    pwm = PWM(Pin(BL))
    pwm.freq(1000)
    pwm.duty_u16(32768)  # max 65535

    keyA = Pin(15, Pin.IN, Pin.PULL_UP)
    keyB = Pin(17, Pin.IN, Pin.PULL_UP)

    lcd = LCD_1inch14()
    # color BRG
    paint.paint_boot(lcd, "Waiting for connection...")
    lcd.show()

    sleep(2)

    while True:
        connection = None
        try:
            ip = connect()
            connection = open_socket(ip)
            paint.paint_ready(lcd, "Ready and accepting requests!", ip)

            serve(connection, lcd)
        except Exception as exc:
            if connection:
                connection.close()
            print(f"An error occurred: {exc}")
            paint.paint_error(lcd, ssid, ERROR_AFTER_SECS, retry=True)
            listen_for_retry_click(lcd, keyA, keyB)


