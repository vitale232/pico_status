def trim(text):
    if len(text) > 28:
        text = text[:25] + "..."
    return text


def paint_boot(lcd, text):
    lcd.fill(lcd.purple)
    lcd.text(text, 2, 20, lcd.white)
    lcd.show()
    return True


def paint_reconnect(lcd, top_text, bottom_text):
    lcd.fill(lcd.purple)
    if top_text:
        lcd.text(trim(top_text), 2, 20, lcd.white)
    if bottom_text:
        lcd.text(trim(bottom_text), 2, 40, lcd.white)
    lcd.show()
    return True


def paint_ready(lcd, text, ip):
    lcd.fill(lcd.pink)
    lcd.text(text, 2, 20, lcd.black)
    lcd.text(f"IPv4: {ip}", 4, 60, lcd.black)
    lcd.show()
    return True


def paint_error(lcd, ssid, error_after_secs, retry=False):
    lcd.fill(lcd.orange)
    lcd.text("CONNECTION FAILED!!!!", 2, 20, lcd.white)
    lcd.text(f"Network: {ssid}", 10, 40, lcd.white)
    lcd.text(f"Waited: {error_after_secs} seconds.", 10, 60, lcd.white)
    lcd.text("Restart Pico to try again!", 2, 80, lcd.white)
    if retry:
        lcd.text("A or B to retry...", 10, 120, lcd.white)
    lcd.show()
    return True


def paint_green(lcd, top_text, bottom_text):
    lcd.fill(lcd.bg_to_hex("GREEN"))
    if top_text:
        lcd.text(trim(top_text), 2, 20, lcd.black)
    if bottom_text:
        lcd.text(trim(bottom_text), 2, 40, lcd.black)
    lcd.show()
    return True


def paint_yellow(lcd, top_text, bottom_text):
    lcd.fill(lcd.bg_to_hex("YELLOW"))
    if top_text:
        lcd.text(trim(top_text), 2, 20, lcd.black)
    if bottom_text:
        lcd.text(trim(bottom_text), 2, 40, lcd.black)
    lcd.show()
    return True


def paint_red(lcd, top_text, bottom_text):
    lcd.fill(lcd.bg_to_hex("RED"))
    if top_text:
        lcd.text(trim(top_text), 2, 20, lcd.white)
    if bottom_text:
        lcd.text(trim(bottom_text), 2, 40, lcd.white)
    lcd.show()
    return True


def paint_state(lcd, state, top_text, bottom_text):
    if state == "GREEN":
        return paint_green(lcd, top_text, bottom_text)
    elif state == "YELLOW":
        return paint_yellow(lcd, top_text, bottom_text)
    elif state == "RED":
        return paint_red(lcd, top_text, bottom_text)
    else:
        print(f"'{state}' is not a valid color state!")
        return paint_red(lcd, "Something went wrong!", f"Couldn't parse: {state}")
