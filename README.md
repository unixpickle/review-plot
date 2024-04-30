# API examples

Fetch search results for "Grand Hyatt"

```bash
curl 'http://localhost:8080/api/search?latitude=37.63&longitude=-122.44&accuracy=10.0&query=Grand+Hyatt'
```

Fetch reviews for a given place:

```bash
curl 'http://localhost:8080/api/reviews??latitude=37.63&longitude=-122.44&accuracy=10.0&url=https%3A%2F%2Fwww.google.com%2Fmaps%2Fplace%2FAirTrain%2BStation%2BGrand%2BHyatt%2Fdata%3D%214m7%213m6%211s0x808f77804262297f%3A0xb04f280673adf4b0%218m2%213d37.6133661%214d-122.3939003%2116s%252Fg%252F11j0qhz7n3%2119sChIJfyliQoB3j4ARsPStcwYoT7A%3Fauthuser%3D0%26hl%3Den%26rclk%3D1'
```
