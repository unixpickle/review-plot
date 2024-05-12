import argparse
import json
import random
from queue import Empty, Queue
from threading import Thread

import requests


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=str, default="locations.json")
    parser.add_argument("--num_workers", type=int, default=16)
    parser.add_argument("--num_subsegs", type=int, default=10)
    args = parser.parse_args()
    in_queue = Queue()
    out_queue = Queue()

    two_bit_segments = []
    for first in range(256):
        for second in range(256):
            two_bit_segments.append(f"{first}.{second}")

    print("feeding queue...")
    for seg in two_bit_segments:
        if seg.startswith("0.") or seg.startswith("255."):
            continue
        for subseg in random.choices(two_bit_segments, k=args.num_subsegs):
            in_queue.put(f"{seg}.{subseg}")

    print("scraping...")

    threads = []
    for _ in range(args.num_workers):
        th = Thread(target=fetch_locations, args=(in_queue, out_queue), daemon=True)
        th.start()
        threads.append(th)
    for th in threads:
        th.join()
    results = {}
    while True:
        try:
            x = out_queue.get_nowait()
        except Empty:
            break
        results[x[0]] = x[1:]
    with open(args.output, "w") as f:
        json.dump(results, f)


def fetch_locations(q: Queue, results: Queue):
    with requests.Session() as sess:
        while True:
            try:
                ip_addr = q.get_nowait()
            except Empty:
                return
            result = sess.get(
                f"https://www.geolocation.com/en_us?ip={ip_addr}", timeout=20
            ).content
            lat_str = b"<div><label><strong>Latitude</strong></label></div>"
            lon_str = b"<div><label><strong>Longitude</strong></label></div>"
            parsed_latlon = []
            for s in [lat_str, lon_str]:
                try:
                    idx = result.index(s)
                except ValueError:
                    print(f"string '{s}' not found for {ip_addr}")
                    break
                try:
                    data = result[idx + len(s) :].split(b"\n")[1]
                    parsed_latlon.append(float(data.strip()))
                except Exception as exc:
                    print(f"failed to parse: {exc} for {ip_addr}")
                    break
            if len(parsed_latlon) != 2:
                continue
            lat, lon = parsed_latlon
            results.put((ip_addr, lat, lon))
            print(f"done {ip_addr}")


if __name__ == "__main__":
    main()
