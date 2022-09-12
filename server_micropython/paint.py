# https://thepihut.com/blogs/raspberry-pi-tutorials/coding-colour-with-micropython-on-raspberry-pi-pico-displays
def color(R, G, B):  # Convert RGB888 to RGB565
    return (
        (((G & 0b00011100) << 3) + ((B & 0b11111000) >> 3) << 8)
        + (R & 0b11111000)
        + ((G & 0b11100000) >> 5)
    )


colors = {
    "red": color(255, 0, 0),
    "yellow": color(255, 255, 0),
    "green": color(0, 255, 0),  #
    "white": color(255, 255, 255),
    "black": color(0, 0, 0),
    "orange": color(204, 132, 0),
    "purple": color(111, 0, 255),
    "pink": color(254, 221, 228),
    "dark_red": color(120, 0, 33),
}


def trim(text):
    if len(text) > 28:
        text = text[:25] + "..."
    return text


def paint_boot(lcd, text):
    lcd.fill(colors["purple"])
    lcd.text(text, 2, 20, colors["white"])
    lcd.show()
    return True


def paint_reconnect(lcd, top_text, bottom_text):
    lcd.fill(colors["purple"])
    lcd.fill(colors['purple'])
    if top_text:
        lcd.text(trim(top_text), 2, 20, colors["white"])
    if bottom_text:
        lcd.text(trim(bottom_text), 2, 40, colors["white"])
    lcd.show()
    return True


def paint_ready(lcd, text, ip):
    lcd.fill(colors["pink"])
    lcd.text(text, 2, 20, colors["black"])
    lcd.text(f"IPv4: {ip}", 4, 60, colors["black"])
    lcd.show()
    return True


def paint_error(lcd, ssid, error_after_secs, retry=False):
    lcd.fill(colors["orange"])
    lcd.text("CONNECTION FAILED!!!!", 2, 20, colors["white"])
    lcd.text(f"Network: {ssid}", 10, 40, colors["white"])
    lcd.text(f"Waited: {error_after_secs} seconds.", 10, 60, colors["white"])
    lcd.text("Restart Pico to try again!", 2, 80, colors["white"])
    if retry:
        lcd.text("A or B to retry...", 10, 120, colors["white"])
    lcd.show()
    return True


def paint_status(
    lcd, color_state, line1=None, line2=None, line3=None, line4=None, line5=None, line6=None, line7=None
):
    print(f'paint_status: color_state={color_state}, lines1,2={line1}{line2}')
    text_color = colors["black"]
    if color_state == "GREEN":
        lcd.fill(colors["green"])
        text_color = colors["black"]
    elif color_state == "YELLOW":
        lcd.fill(colors["yellow"])
        text_color = colors["black"]
    elif color_state == "RED":
        lcd.fill(colors["red"])
        text_color = colors["white"]
    elif color_state == "DARK_RED":
        lcd.fill(colors["dark_red"])
        text_color = colors["white"]
    else:
        lcd.fill(colors["pink"])
        text_color = colors["black"]

    if line1:
        lcd.text(trim(line1), 2, 0, text_color)
    if line2:
        lcd.text(trim(line2), 2, 20, text_color)
    if line3:
        lcd.text(trim(line3), 2, 40, text_color)
    if line4:
        lcd.text(trim(line4), 2, 60, text_color)
    if line5:
        lcd.text(trim(line5), 2, 80, text_color)
    if line6:
        lcd.text(trim(line6), 2, 100, text_color)
    if line7:
        lcd.text(trim(line7), 2, 120, text_color)

    lcd.show()
    return color_state
