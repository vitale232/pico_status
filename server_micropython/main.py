from machine import Pin, SPI, PWM
import socket
import network
import framebuf
from time import sleep

from secrets import ssid, password


BL = 13
DC = 8
RST = 12
MOSI = 11
SCK = 10
CS = 9


ERROR_AFTER_SECS = 10


def cels_to_fahr(temp, prcsn=2):
    return round((temp * (9 / 5)) + 32, prcsn)


# https://thepihut.com/blogs/raspberry-pi-tutorials/coding-colour-with-micropython-on-raspberry-pi-pico-displays
def color(R, G, B):  # Convert RGB888 to RGB565
    return (
        (((G & 0b00011100) << 3) + ((B & 0b11111000) >> 3) << 8)
        + (R & 0b11111000)
        + ((G & 0b11100000) >> 5)
    )


class LCD_1inch14(framebuf.FrameBuffer):
    def __init__(self):
        self.width = 240
        self.height = 135

        self.cs = Pin(CS, Pin.OUT)
        self.rst = Pin(RST, Pin.OUT)

        self.cs(1)
        self.spi = SPI(1)
        self.spi = SPI(1, 1000_000)
        self.spi = SPI(
            1, 10000_000, polarity=0, phase=0, sck=Pin(SCK), mosi=Pin(MOSI), miso=None
        )
        self.dc = Pin(DC, Pin.OUT)
        self.dc(1)
        self.buffer = bytearray(self.height * self.width * 2)
        super().__init__(self.buffer, self.width, self.height, framebuf.RGB565)
        self.init_display()

        self.red = color(255, 0, 0)
        self.yellow = color(255, 255, 0)
        self.green = color(0, 255, 0)  #

        self.white = color(255, 255, 255)  # 0xffff
        self.black = color(0, 0, 0)

        self.orange = color(204, 132, 0)
        self.purple = color(111, 0, 255)
        self.pink = color(254, 221, 228)

        self.bg_color = "GREEN"  # "RED" or "GREEN" or "YELLOW"

    def bg_to_hex(self, bg):
        if bg == "GREEN":
            self.bg_color = "GREEN"
            return self.green
        elif bg == "YELLOW":
            self.bg_color = "YELLOW"
            return self.yellow
        elif bg == "RED":
            self.bg_color = "RED"
            return self.red
        else:
            return self.black

    def write_cmd(self, cmd):
        self.cs(1)
        self.dc(0)
        self.cs(0)
        self.spi.write(bytearray([cmd]))
        self.cs(1)

    def write_data(self, buf):
        self.cs(1)
        self.dc(1)
        self.cs(0)
        self.spi.write(bytearray([buf]))
        self.cs(1)

    def init_display(self):
        """Initialize dispaly"""
        self.rst(1)
        self.rst(0)
        self.rst(1)

        self.write_cmd(0x36)
        self.write_data(0x70)

        self.write_cmd(0x3A)
        self.write_data(0x05)

        self.write_cmd(0xB2)
        self.write_data(0x0C)
        self.write_data(0x0C)
        self.write_data(0x00)
        self.write_data(0x33)
        self.write_data(0x33)

        self.write_cmd(0xB7)
        self.write_data(0x35)

        self.write_cmd(0xBB)
        self.write_data(0x19)

        self.write_cmd(0xC0)
        self.write_data(0x2C)

        self.write_cmd(0xC2)
        self.write_data(0x01)

        self.write_cmd(0xC3)
        self.write_data(0x12)

        self.write_cmd(0xC4)
        self.write_data(0x20)

        self.write_cmd(0xC6)
        self.write_data(0x0F)

        self.write_cmd(0xD0)
        self.write_data(0xA4)
        self.write_data(0xA1)

        self.write_cmd(0xE0)
        self.write_data(0xD0)
        self.write_data(0x04)
        self.write_data(0x0D)
        self.write_data(0x11)
        self.write_data(0x13)
        self.write_data(0x2B)
        self.write_data(0x3F)
        self.write_data(0x54)
        self.write_data(0x4C)
        self.write_data(0x18)
        self.write_data(0x0D)
        self.write_data(0x0B)
        self.write_data(0x1F)
        self.write_data(0x23)

        self.write_cmd(0xE1)
        self.write_data(0xD0)
        self.write_data(0x04)
        self.write_data(0x0C)
        self.write_data(0x11)
        self.write_data(0x13)
        self.write_data(0x2C)
        self.write_data(0x3F)
        self.write_data(0x44)
        self.write_data(0x51)
        self.write_data(0x2F)
        self.write_data(0x1F)
        self.write_data(0x1F)
        self.write_data(0x20)
        self.write_data(0x23)

        self.write_cmd(0x21)

        self.write_cmd(0x11)

        self.write_cmd(0x29)

    def show(self):
        self.write_cmd(0x2A)
        self.write_data(0x00)
        self.write_data(0x28)
        self.write_data(0x01)
        self.write_data(0x17)

        self.write_cmd(0x2B)
        self.write_data(0x00)
        self.write_data(0x35)
        self.write_data(0x00)
        self.write_data(0xBB)

        self.write_cmd(0x2C)

        self.cs(1)
        self.dc(1)
        self.cs(0)
        self.spi.write(self.buffer)
        self.cs(1)


def trim(text):
    if len(text) > 28:
        text = text[:25] + "..."
    return text


def paint_boot(LCD, text):
    LCD.fill(LCD.purple)
    LCD.text(text, 2, 20, LCD.white)
    LCD.show()
    return True

def paint_reconnect(LCD, top_text, bottom_text):
    LCD.fill(LCD.purple)
    if top_text:
        LCD.text(trim(top_text), 2, 20, LCD.white)
    if bottom_text:
        LCD.text(trim(bottom_text), 2, 40, LCD.white)
    LCD.show()
    return True


def paint_ready(LCD, text, ip):
    LCD.fill(LCD.pink)
    LCD.text(text, 2, 20, LCD.black)
    LCD.text(f"IPv4: {ip}", 4, 60, LCD.black)
    LCD.show()
    return True


def paint_error(LCD, retry=False):
    LCD.fill(LCD.orange)
    LCD.text("CONNECTION FAILED!!!!", 2, 20, LCD.white)
    LCD.text(f"Network: {ssid}", 10, 40, LCD.white)
    LCD.text(f"Waited: {ERROR_AFTER_SECS} seconds.", 10, 60, LCD.white)
    LCD.text("Restart Pico to try again!", 2, 80, LCD.white)
    if retry:
        LCD.text("A or B to retry...", 10, 120, LCD.white)
    LCD.show()
    return True


def paint_green(LCD, top_text, bottom_text):
    LCD.fill(LCD.bg_to_hex("GREEN"))
    if top_text:
        LCD.text(trim(top_text), 2, 20, LCD.black)
    if bottom_text:
        LCD.text(trim(bottom_text), 2, 40, LCD.black)
    LCD.show()
    return True


def paint_yellow(LCD, top_text, bottom_text):
    LCD.fill(LCD.bg_to_hex("YELLOW"))
    if top_text:
        LCD.text(trim(top_text), 2, 20, LCD.black)
    if bottom_text:
        LCD.text(trim(bottom_text), 2, 40, LCD.black)
    LCD.show()
    return True


def paint_red(LCD, top_text, bottom_text):
    LCD.fill(LCD.bg_to_hex("RED"))
    if top_text:
        LCD.text(trim(top_text), 2, 20, LCD.white)
    if bottom_text:
        LCD.text(trim(bottom_text), 2, 40, LCD.white)
    LCD.show()
    return True


def paint_state(LCD, state, top_text, bottom_text):
    if state == "GREEN":
        return paint_green(LCD, top_text, bottom_text)
    elif state == "YELLOW":
        return paint_yellow(LCD, top_text, bottom_text)
    elif state == "RED":
        return paint_red(LCD, top_text, bottom_text)
    else:
        print(f"'{state}' is not a valid color state!")
        return paint_red(LCD, "Something went wrong!", f"Couldn't parse: {state}")


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
    try:
        req = req.split()[1]
    except IndexError:
        pass
    url_parts = req.split("?")
    top_text = ""
    bottom_text = ""
    if len(url_parts) == 0:
        return state
    if len(url_parts) == 1:
        req = url_parts[0]
    if len(url_parts) == 2:
        req = url_parts[0]
        (top_text, bottom_text) = parse_text(url_parts[1])
    if req == "/green":
        state = "GREEN"
    if req == "/yellow":
        state = "YELLOW"
    if req == "/red":
        state = "RED"

    return (state, top_text, bottom_text)


def parse_text(text):
    text = text.replace("?", "").split("&")
    top_text = ""
    bottom_text = ""
    for el in text:
        (param, val) = el.split("=")
        if param == "top_text":
            top_text = unquote(val)
        if param == "bottom_text":
            bottom_text = unquote(val)
    return (top_text, bottom_text)


def render(temperature, state):
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


def serve(connection, LCD):
    print("Staring server...")
    color_state = "GREEN"
    while True:
        client = connection.accept()[0]
        request = client.recv(1024)
        request = str(request)
        print(f"Incoming Request:\n{request}")
        try:
            (color_state, top_text, bottom_text) = parse_request(request, color_state)
            paint_state(LCD, color_state, top_text, bottom_text)
            html = render(0, color_state)
            client.send(html)
            client.close()
        except Exception:
            print("Could not parse request")
            client.close()


def listen_for_retry_click(LCD, a_button, b_button):
    while True:
        if a_button.value() == 0:
            paint_reconnect(LCD, "Trying new server...", "If this fails, reboot!")
            sleep(2)
            return
        if b_button.value() == 0:
            paint_reconnect(LCD, "Trying to re-establish server...", "If this fails, reboot!")
            sleep(2)
            return


if __name__ == "__main__":
    pwm = PWM(Pin(BL))
    pwm.freq(1000)
    pwm.duty_u16(32768)  # max 65535

    keyA = Pin(15, Pin.IN, Pin.PULL_UP)
    keyB = Pin(17, Pin.IN, Pin.PULL_UP)

    LCD = LCD_1inch14()
    # color BRG
    paint_boot(LCD, "Waiting for connection...")
    LCD.show()

    sleep(2)

    while True:
        connection = None
        try:
            ip = connect()
            connection = open_socket(ip)
            paint_ready(LCD, "Ready and accepting requests!", ip)

            serve(connection, LCD)
        except Exception as exc:
            if connection:
                connection.close()
            print(f"An error occurred: {exc}")
            paint_error(LCD, retry=True)
            listen_for_retry_click(LCD, keyA, keyB)
