#!/usr/bin/env python3
"""Fetch HKIA passenger departures and send them to the CYD over USB serial."""

import argparse
import datetime as dt
import glob
import json
import os
import re
import select
import sys
import termios
import time
import tty
import unicodedata
from concurrent.futures import ThreadPoolExecutor
from urllib.request import Request, urlopen


HKT = dt.timezone(dt.timedelta(hours=8))
API = "https://www.hongkongairport.com/flightinfo-rest/rest/flights/past"
ROWS = 8
WINDOW = dt.timedelta(hours=4)
RETRY_SECONDS = 15
ESTIMATE = re.compile(r"\b(\d{1,2}:\d{2})(?:\s*\((\d{2}/\d{2}/\d{4})\))?")
CITIES = {
    "ADD": ("Addis Ababa", "亞的斯"), "AKL": ("Auckland", "奧克蘭"),
    "AMS": ("Amsterdam", "阿姆斯特丹"), "AUH": ("Abu Dhabi", "阿布扎比"),
    "BKI": ("Kota Kinabalu", "亞庇"), "BKK": ("Bangkok", "曼谷"),
    "BLR": ("Bengaluru", "班加羅爾"), "BNE": ("Brisbane", "布里斯班"),
    "BOM": ("Mumbai", "孟買"), "BOS": ("Boston", "波士頓"),
    "BRU": ("Brussels", "布魯塞爾"), "BWN": ("Bandar Seri", "斯里巴加灣"),
    "CAN": ("Guangzhou", "廣州"), "CDG": ("Paris", "巴黎"),
    "CEB": ("Cebu", "宿霧"), "CGK": ("Jakarta", "雅加達"),
    "CHC": ("Christchurch", "基督城"), "CJU": ("Jeju", "濟州"),
    "CKG": ("Chongqing", "重慶"), "CMB": ("Colombo", "科倫坡"),
    "CNX": ("Chiang Mai", "清邁"), "CRK": ("Clark", "克拉克"),
    "CSX": ("Changsha", "長沙"), "CTS": ("Sapporo", "札幌"),
    "CTU": ("Chengdu/CTU", "成都雙流"), "CZX": ("Changzhou", "常州"),
    "DAD": ("Da Nang", "峴港"), "DEL": ("Delhi", "德里"),
    "DFW": ("Dallas", "達拉斯"), "DMK": ("Bangkok/DMK", "曼谷廊曼"),
    "DOH": ("Doha", "多哈"), "DPS": ("Denpasar", "峇里"),
    "DVO": ("Davao", "達沃"), "DXB": ("Dubai", "杜拜"),
    "FOC": ("Fuzhou", "福州"), "FRA": ("Frankfurt", "法蘭克福"),
    "FUK": ("Fukuoka", "福岡"), "HAK": ("Haikou", "海口"),
    "HAN": ("Hanoi", "河內"), "HEL": ("Helsinki", "赫爾辛基"),
    "HET": ("Hohhot", "呼和浩特"), "HFE": ("Hefei", "合肥"),
    "HGH": ("Hangzhou", "杭州"), "HIJ": ("Hiroshima", "廣島"),
    "HKT": ("Phuket", "布吉"), "HND": ("Tokyo/HND", "東京羽田"),
    "HYD": ("Hyderabad", "海得拉巴"), "ICN": ("Seoul/ICN", "首爾仁川"),
    "IST": ("Istanbul", "伊斯坦堡"), "JFK": ("New York/JFK", "紐約甘迺迪"),
    "JNB": ("Johannesburg", "約翰內斯堡"), "KHH": ("Kaohsiung", "高雄"),
    "KIX": ("Osaka", "大阪"), "KMG": ("Kunming", "昆明"),
    "KTI": ("Phnom Penh", "金邊"), "KTM": ("Kathmandu", "加德滿都"),
    "KUL": ("Kuala Lumpur", "吉隆坡"), "LAX": ("Los Angeles", "洛杉磯"),
    "LHR": ("London/LHR", "倫敦希斯路"), "LJG": ("Lijiang", "麗江"),
    "MAA": ("Chennai", "欽奈"), "MAD": ("Madrid", "馬德里"),
    "MAN": ("Manchester", "曼徹斯特"), "MEL": ("Melbourne", "墨爾本"),
    "MNL": ("Manila", "馬尼拉"), "MXP": ("Milan", "米蘭"),
    "NGB": ("Ningbo", "寧波"), "NGO": ("Nagoya", "名古屋"),
    "NKG": ("Nanjing", "南京"), "NNG": ("Nanning", "南寧"),
    "NRT": ("Tokyo/NRT", "東京成田"), "OKA": ("Okinawa", "沖繩"),
    "ORD": ("Chicago", "芝加哥"), "PEK": ("Beijing/PEK", "北京首都"),
    "PEN": ("Penang", "檳城"), "PER": ("Perth", "珀斯"),
    "PKX": ("Beijing/PKX", "北京大興"), "POM": ("Port Moresby", "莫爾茲比港"),
    "PQC": ("Phu Quoc", "富國島"), "PUS": ("Busan", "釜山"),
    "PVG": ("Shanghai/PVG", "上海浦東"), "RMQ": ("Taichung", "台中"),
    "SDJ": ("Sendai", "仙台"), "SEA": ("Seattle", "西雅圖"),
    "SFO": ("San Francisco", "三藩市"), "SGN": ("Ho Chi Minh", "胡志明市"),
    "SHA": ("Shanghai/SHA", "上海虹橋"), "SIN": ("Singapore", "新加坡"),
    "SUB": ("Surabaya", "泗水"), "SVO": ("Moscow/SVO", "莫斯科"),
    "SYD": ("Sydney", "悉尼"), "SZB": ("Subang", "梳邦"),
    "TAK": ("Takamatsu", "高松"), "TAO": ("Qingdao", "青島"),
    "TFU": ("Chengdu/TFU", "成都天府"), "TNA": ("Jinan", "濟南"),
    "TPE": ("Taipei", "台北"), "TSN": ("Tianjin", "天津"),
    "UBN": ("Ulaanbaatar", "烏蘭巴托"), "USM": ("Koh Samui", "蘇梅島"),
    "WNZ": ("Wenzhou", "溫州"), "WUH": ("Wuhan", "武漢"),
    "WUX": ("Wuxi", "無錫"), "XIY": ("Xi'an", "西安"),
    "XMN": ("Xiamen", "廈門"), "YIH": ("Yichang", "宜昌"),
    "YIW": ("Yiwu", "義烏"), "YVR": ("Vancouver", "溫哥華"),
    "YYZ": ("Toronto", "多倫多"), "ZRH": ("Zurich", "蘇黎世"),
}
STATUS_ZH = {
    "Boarding": "現正登機",
    "Boarding soon": "即將登機",
    "Final call": "最後召集",
    "Gate open": "閘口開放",
    "Gate closed": "截止登機",
    "Cancelled": "取消",
    "Delayed": "延誤",
    "": "-",
}


def fetch_day(day):
    url = f"{API}?date={day}&lang=en&cargo=false&arrival=false"
    request = Request(url, headers={"User-Agent": "CYD HKG flight board/1.0"})
    with urlopen(request, timeout=12) as response:
        data = json.load(response)
    if not isinstance(data, list) or not any(batch.get("date") == str(day) for batch in data):
        raise ValueError(f"Airport feed has no schedule for {day}")
    return data


def get_departures(batches, now):
    window_start = now.replace(second=0, microsecond=0)
    window_end = now + WINDOW
    departures = {}
    for batch in batches:
        day = dt.date.fromisoformat(batch["date"])
        for flight in batch["list"]:
            numbers = flight.get("flight") or []
            if not numbers or not numbers[0].get("no"):
                continue
            number = numbers[0]["no"]
            scheduled = dt.datetime.combine(day, dt.time.fromisoformat(flight["time"]), HKT)
            gate = str(flight.get("gate") or "").strip()
            status = (flight.get("status") or "").strip()
            if gate in {"-", "—"}:
                gate = ""
            if status in {"-", "—"}:
                status = ""
            if not gate and not status:
                continue
            status_lower = status.casefold()
            if status_lower.startswith(("dep ", "departed")):
                continue

            effective_time = scheduled
            if status_lower.startswith(("est ", "estimated")) and (match := ESTIMATE.search(status)):
                clock, date = match.groups()
                estimate_day = dt.datetime.strptime(date, "%d/%m/%Y").date() if date else day
                estimate = dt.datetime.combine(estimate_day, dt.datetime.strptime(clock, "%H:%M").time(), HKT)
                if date is None and estimate < scheduled - dt.timedelta(hours=12):
                    estimate += dt.timedelta(days=1)
                effective_time = estimate
                status = f"Est {estimate:%H:%M}"

            if not window_start <= effective_time <= window_end:
                continue
            if status_lower.startswith("cancel"):
                status = "Cancelled"
            elif status_lower.startswith("delay"):
                status = "Delayed"
            elif status_lower in {"boarding", "boarding soon", "final call", "gate open", "gate closed"}:
                status = status_lower.capitalize()

            codes = flight.get("destination") or []
            code = codes[0] if codes else "-"
            destination_en, destination_zh = CITIES.get(code, (code, code))
            status_zh = f"預計 {status[4:]}" if status.startswith("Est ") else STATUS_ZH.get(status, "請查詢")
            departures[(scheduled, number)] = (
                effective_time,
                (f"{scheduled:%H:%M}", number, destination_en, gate, status,
                 destination_zh, status_zh),
            )

    return [row for _, row in sorted(departures.values(), key=lambda item: (item[0], item[1][1]))]


def paginate(departures):
    pages = [departures[start:start + ROWS] for start in range(0, len(departures), ROWS)]
    if len(pages) > 1:
        pages[-1] = departures[-ROWS:]
    return pages or [[]]


def field(value, pixel_width, chinese=False):
    text = unicodedata.normalize("NFC" if chinese else "NFKD", str(value or "-"))
    text = " ".join(text.replace("|", " ").split())
    chars = []
    for char in text:
        if " " <= char <= "~":
            width = 6
        elif chinese and "\u3400" <= char <= "\u9fff":
            width = 16
        else:
            continue
        if pixel_width < width:
            break
        chars.append(char)
        pixel_width -= width
    return "".join(chars).rstrip() or "-"


def packet(now, page, page_count, departures):
    fields = ["HKG3", now.strftime("%H:%M:%S"), f"{page}/{page_count}"]
    for time_text, flight, destination, gate, status, destination_zh, status_zh in departures:
        fields.append("\t".join((
            time_text, field(flight, 54), field(destination, 78), field(gate, 24),
            field(status, 84), field(destination_zh, 80, chinese=True),
            field(status_zh, 84, chinese=True),
        )))
    return ("|".join(fields) + "\n").encode("utf-8")


def send(port, payload):
    fd = os.open(port, os.O_RDWR | os.O_NOCTTY | os.O_NONBLOCK)
    try:
        tty.setraw(fd)
        settings = termios.tcgetattr(fd)
        settings[2] |= termios.CLOCAL | termios.CREAD
        settings[2] &= ~termios.HUPCL
        settings[4] = settings[5] = termios.B115200
        termios.tcsetattr(fd, termios.TCSANOW, settings)

        for _ in range(2):
            termios.tcflush(fd, termios.TCIFLUSH)
            deadline = time.monotonic() + 5
            pending = memoryview(b"\n" + payload)
            while pending:
                _, writable, _ = select.select([], [fd], [], max(0, deadline - time.monotonic()))
                if not writable:
                    raise TimeoutError("Timed out sending the flight board update")
                pending = pending[os.write(fd, pending):]

            reply = b""
            page_started = False
            while time.monotonic() < deadline:
                ready, _, _ = select.select([fd], [], [], max(0, deadline - time.monotonic()))
                if not ready:
                    break
                reply = (reply + os.read(fd, 512))[-512:]
                if not page_started and b"HKG VIEW en\n" in reply:
                    page_started = True
                    deadline = time.monotonic() + 14
                if b"HKG VIEW en\n" in reply and b"HKG VIEW zh-HK\n" in reply and b"HKG DONE\n" in reply:
                    return
        raise TimeoutError("ESP32 did not complete the bilingual page")
    finally:
        os.close(fd)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--port", help="USB serial device, e.g. /dev/cu.usbserial-10")
    parser.add_argument("--once", action="store_true", help="Show every page of one snapshot and exit")
    parser.add_argument("--preview", action="store_true", help="Print the current page packets without using USB")
    args = parser.parse_args()
    if not args.port and not args.preview:
        ports = glob.glob("/dev/cu.usbserial-*") + glob.glob("/dev/ttyUSB*")
        if len(ports) != 1:
            parser.error("Connect the board and specify its USB device with --port")
        args.port = ports[0]

    with ThreadPoolExecutor(max_workers=2) as pool:
        while True:
            now = dt.datetime.now(HKT)
            until = (now + WINDOW).date()
            dates = [now.date()] if now.date() == until else [now.date(), until]
            try:
                batches = [batch for response in pool.map(fetch_day, dates) for batch in response]
                departures = get_departures(batches, now)
            except (OSError, ValueError, KeyError, TypeError) as error:
                print(f"{now:%H:%M:%S} HKT  {error}", file=sys.stderr, flush=True)
                if args.once or args.preview:
                    return 1
                time.sleep(RETRY_SECONDS)
                continue

            pages = paginate(departures)
            received = sum(len(batch["list"]) for batch in batches)
            print(f"{now:%H:%M:%S} HKT  JSON {received} rows, {len(departures)} matched, {len(pages)} pages",
                  file=sys.stderr, flush=True)
            for index, rows in enumerate(pages, 1):
                payload = packet(now, index, len(pages), rows)
                if args.preview:
                    print(payload.decode("utf-8"), end="")
                    continue
                while True:
                    try:
                        send(args.port, payload)
                        break
                    except OSError as error:
                        print(error, file=sys.stderr, flush=True)
                        if args.once:
                            return 1
                        time.sleep(RETRY_SECONDS)
                finished = dt.datetime.now(HKT)
                print(f"{finished:%H:%M:%S} HKT  ESP32 page {index}/{len(pages)} completed EN + 繁中 ({len(rows)} rows)",
                      flush=True)
            if args.once or args.preview:
                return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except KeyboardInterrupt:
        sys.exit(0)
