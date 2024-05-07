var __awaiter = (this && this.__awaiter) || function (thisArg, _arguments, P, generator) {
    function adopt(value) { return value instanceof P ? value : new P(function (resolve) { resolve(value); }); }
    return new (P || (P = Promise))(function (resolve, reject) {
        function fulfilled(value) { try { step(generator.next(value)); } catch (e) { reject(e); } }
        function rejected(value) { try { step(generator["throw"](value)); } catch (e) { reject(e); } }
        function step(result) { result.done ? resolve(result.value) : adopt(result.value).then(fulfilled, rejected); }
        step((generator = generator.apply(thisArg, _arguments || [])).next());
    });
};
class ReviewPlot {
    constructor() {
        this.items = [];
        this._element = document.createElement('div');
        this._element.className = 'plot';
        this.startDate = document.createElement('label');
        this.startDate.className = 'plot-start-date';
        this.endDate = document.createElement('label');
        this.endDate.className = 'plot-end-date';
        this.dots = document.createElement('div');
        this.dots.className = 'plot-dots';
        this._element.appendChild(this.startDate);
        this._element.appendChild(this.endDate);
        this._element.appendChild(this.dots);
        this.updateUI();
    }
    element() {
        return this._element;
    }
    clearItems() {
        this.items = [];
        this.updateUI();
    }
    addItems(items) {
        this.items = this.items.concat(items);
        this.items.sort((x, y) => x.timestamp - y.timestamp);
        this.updateUI();
    }
    updateUI() {
        this.dots.textContent = '';
        if (this.items.length == 0) {
            this.startDate.textContent = 'No data';
            this.endDate.textContent = 'No data';
            return;
        }
        const start = new Date(this.items[0].timestamp * 1000);
        const end = new Date(this.items[this.items.length - 1].timestamp * 1000);
        this.startDate.textContent = `${start}`;
        this.endDate.textContent = `${end}`;
        const span = Math.max(1, this.items[this.items.length - 1].timestamp - this.items[0].timestamp);
        this.items.forEach((x) => {
            const timestamp = x.timestamp;
            const frac = this.items.length > 1 ? (timestamp - this.items[0].timestamp) / span : 0.5;
            const dot = document.createElement('div');
            dot.className = 'plot-dots-dot';
            dot.style.left = marginPercent(frac);
            dot.style.bottom = marginPercent((x.rating - 1) / 4);
            this.dots.appendChild(dot);
        });
    }
}
function marginPercent(frac) {
    return `${(frac * 90 + 5).toFixed(3)}%`;
}
class ReviewQuery {
    constructor(url) {
        this.url = url;
        this.onResults = null;
        this.onError = null;
        this.onDone = null;
        this.abort = new AbortController();
    }
    run() {
        return __awaiter(this, void 0, void 0, function* () {
            const place = window.app.urlEncodeLocation();
            const url = `/api/reviews?url=${encodeURIComponent(this.url)}&${place}`;
            try {
                const results = yield fetch(url);
                const reader = results.body.getReader();
                let buf = '';
                while (true) {
                    let result = yield reader.read();
                    if (result.done) {
                        this.onDone();
                        return;
                    }
                    buf += new TextDecoder().decode(result.value);
                    while (buf.includes('\n')) {
                        const lineIndex = buf.indexOf('\n');
                        const line = buf.substring(0, lineIndex);
                        buf = buf.substring(lineIndex + 1);
                        const parsed = JSON.parse(line);
                        this.onResults(parsed);
                    }
                }
            }
            catch (e) {
                this.onError(e.toString());
            }
        });
    }
    cancel() {
        this.onResults = (_) => null;
        this.onError = (_) => null;
        this.onDone = () => null;
        this.abort.abort();
    }
}
//# sourceMappingURL=plot.js.map