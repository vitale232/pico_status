from time import sleep

import socket
import network

import paint
from secrets import ssid, password


def connect(wait_for_conn_secs):
    # Connect to WLAN
    wlan = network.WLAN(network.STA_IF)
    wlan.active(True)
    wlan.connect(ssid, password)

    attempts = 0
    while wlan.isconnected() == False:
        print("Waiting for connection...")
        sleep(1)
        attempts += 1
        if attempts > wait_for_conn_secs:
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


def serve(connection, lcd):
    print("Staring server...")
    color_state = "GREEN"
    while True:
        client = connection.accept()[0]
        request = client.recv(1024)
        request = str(request)
        print(f"Incoming Request:\n{request}")
        html = ""
        try:
            if not is_supported_url(request):
                html = render404()
            else:
                color_state = paint.paint_status(
                    lcd, *parse_request(request, color_state)
                )
                html = render(color_state)
            print(f"html={html}")
            client.send(html)
            client.close()
        except Exception as exc:
            print(f"An exception occurred: {exc}")
            client.close()


def is_supported_url(request):
    url = request.split()[1].lower()
    return (
        url.startswith("/red")
        or url.startswith("/yellow")
        or url.startswith("/green")
        or url.startswith("/late")
    )


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
    if path == "/late":
        state = "DARK_RED"

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

<!DOCTYPE html><html lang='en'><head><meta charset='UTF-8' /><meta http-equiv='X-UA-Compatible' content='IE=edge' /><meta name='viewport' content='width=device-width, initial-scale=1.0' /><title>Pico Status</title></head><body><a href='/green'>Green</a><br /><br /><a href='/yellow'>Yellow</a><br /><br /><a href='/red'>Red</a><p>Screen is {state}</p></body></html>
"""


def render404():
    return """
HTTP/1.1 404 Not Found
Cache-Control: no-cache
Server: pi-in-the-sky
Content-Type: text/html

<!DOCTYPE html><html lang='en'><head><meta charset='UTF-8' /><meta http-equiv='X-UA-Compatible' content='IE=edge' /><meta name='viewport' content='width=device-width, initial-scale=1.0' /><title>Pico Status</title></head><body><h1>Not Found</h1><p>The URL you submitted does not exist on da lil server.</p></body></html>
"""


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
