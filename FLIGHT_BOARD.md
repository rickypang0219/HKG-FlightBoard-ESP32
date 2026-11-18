# HKG 客運離港航班牌

ESP32-2432S028 用 Rust 畫出 320×240 橫向航班牌。Mac 以香港時間篩選未來四小時嘅客運離港航班，有預計起飛時間就用預計時間篩選。已起飛、或者閘口同狀態都空白嘅航班會略過。常見目的地顯示城市名，其餘顯示機場代碼。

每頁八行，英文顯示五秒，繁體中文顯示五秒，再跳下一頁。最後一頁不足八班時會用前頁尾段補滿。整批資料合計不足八班時，餘下行留空。LCD 底部顯示頁碼同今輪查詢時間。

Mac 播完一批資料先再透過 HTTPS 向機管局查詢，四小時範圍跨午夜就額外查下一日。資料經 USB 傳畀 ESP32。[機管局 JSON 資料規格](https://www.hongkongairport.com/iwov-resources/misc/opendata/Flight_Information_DataSpec_en.pdf)

請保持 USB 接駁同程式運行。停收資料三分鐘後，屏幕會保留上次航班同時間，並顯示 `Feed paused` 或 `更新暫停`。[香港機場客運離港航班](https://www.hongkongairport.com/en/flights/departures/passenger.page) · [機管局資料規格](https://www.hongkongairport.com/iwov-resources/misc/opendata/Flight_Information_DataSpec_en.pdf)

## 編譯及燒錄

```sh
source "$HOME/export-esp.sh"
cargo build --bin flight_board --locked
espflash flash --chip esp32 --port /dev/cu.usbserial-10 target/xtensa-esp32-none-elf/debug/flight_board
```

預設 LCD 驅動係 ILI9341。如果確認你嘅板用 ST7789，編譯時加上 `--features st7789`，然後重新燒錄。燒錄後可用 `espflash list-ports` 查 USB 裝置名。

## 更新航班

用 Python 3.10 或以上執行以下指令。Terminal 會顯示原始 JSON 記錄數、篩選後嘅航班數、總頁數同每頁完成時間。按 `Ctrl+C` 可以停止更新。

```sh
python3 flight_feed.py --port /dev/cu.usbserial-10
```

加 `--once` 可以只播完一批資料。用 `python3 flight_feed.py --preview` 可以睇分頁結果。
