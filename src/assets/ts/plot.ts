interface ReviewItem {
    timestamp: number;
    author: string;
    content: string;
    rating: number;
}

class ReviewPlot {
    private items: ReviewItem[] = [];

    private _element: HTMLElement;
    private startDate: HTMLLabelElement;
    private endDate: HTMLLabelElement;
    private dots: HTMLDivElement;
    private statusName: HTMLLabelElement;
    private statusCount: HTMLLabelElement;
    private statusError: HTMLLabelElement;

    private query: ReviewQuery;

    constructor() {
        this._element = document.createElement('div');
        this._element.className = 'plot plot-empty';
        this.startDate = document.createElement('label');
        this.startDate.className = 'plot-start-date';
        this.endDate = document.createElement('label');
        this.endDate.className = 'plot-end-date';
        this.dots = document.createElement('div');
        this.dots.className = 'plot-dots';

        const status = document.createElement('div');
        status.className = 'plot-status';
        this.statusName = document.createElement('label');
        this.statusName.textContent = 'No location selected';
        this.statusName.className = 'plot-status-name';
        this.statusCount = document.createElement('label');
        this.statusCount.className = 'plot-status-count';
        this.statusError = document.createElement('label');
        this.statusError.className = 'plot-status-error';
        status.appendChild(this.statusName);
        status.appendChild(this.statusCount);
        status.appendChild(this.statusError);

        this._element.appendChild(this.startDate);
        this._element.appendChild(this.endDate);
        this._element.appendChild(this.dots);
        this._element.appendChild(status);

        this.updateUI();
    }

    element(): HTMLElement {
        return this._element;
    }

    startQuery(name: string, url: string) {
        if (this.query) {
            this.query.cancel();
            this.query = null;
        }
        this.setStatus('loading');
        this.clearItems();
        this.statusName.textContent = name;
        this.statusCount.textContent = '0 results';
        this.query = new ReviewQuery(url);
        this.query.onDone = () => {
            this.setStatus('loaded');
            this.query = null;
        };
        this.query.onError = (e) => {
            this.statusError.textContent = e.toString();
            this.setStatus('error');
            this.query = null;
        };
        let count = 0;
        this.query.onResults = (results) => {
            count += results.length;
            this.addItems(results);
            this.statusCount.textContent = `${count} results`;
            this.query = null;
        };
        this.query.run();
    }

    private setStatus(status: string) {
        for (let i = 0; i < this._element.classList.length; i++) {
            const cls = this._element.classList[i];
            if (cls.startsWith('plot-')) {
                this._element.classList.remove(cls);
                break;
            }
        }
        this._element.classList.add(`plot-${status}`);
    }

    private clearItems() {
        this.items = [];
        this.updateUI();
    }

    private addItems(items: ReviewItem[]) {
        this.items = this.items.concat(items);
        this.items.sort((x, y) => x.timestamp - y.timestamp);
        this.updateUI();
    }

    private updateUI() {
        this.dots.textContent = '';

        if (this.items.length == 0) {
            this.startDate.textContent = 'No data';
            this.endDate.textContent = 'No data';
            return;
        }

        const start = new Date(this.items[0].timestamp * 1000);
        const end = new Date(this.items[this.items.length - 1].timestamp * 1000);
        this.startDate.textContent = formatDate(start);
        this.endDate.textContent = formatDate(end);

        const span = Math.max(
            1,
            this.items[this.items.length - 1].timestamp - this.items[0].timestamp,
        );

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

function formatDate(date: Date): string {
    let month = (date.getMonth() + 1).toString();
    let day = date.getDate().toString();
    var year = date.getFullYear().toString();
    month = (month.length == 1) ? '0' + month : month;
    day = (day.length == 1) ? '0' + day : day;
    return `${month}/${day}/${year}`;
}

function marginPercent(frac: number): string {
    return `${(frac * 90 + 5).toFixed(3)}%`;
}

interface ErrorResponse {
    error: string;
}

type ReviewResponse = ReviewItem[] | ErrorResponse;

class ReviewQuery {
    public onResults: (_: ReviewItem[]) => void = null;
    public onError: (_: string) => void = null;
    public onDone: () => void = null;

    private abort: AbortController = new AbortController();

    constructor(private url: string) {
    }

    async run() {
        const place = window.app.urlEncodeLocation();
        const url = `/api/reviews?url=${encodeURIComponent(this.url)}&${place}`;
        try {
            const results = await fetch(url);
            const reader = results.body.getReader();
            let buf = '';
            while (true) {
                let result = await reader.read();
                if (result.done) {
                    this.onDone();
                    return;
                }
                buf += new TextDecoder().decode(result.value);
                while (buf.includes('\n')) {
                    const lineIndex = buf.indexOf('\n');
                    const line = buf.substring(0, lineIndex);
                    buf = buf.substring(lineIndex + 1);
                    const parsed: ReviewResponse = JSON.parse(line);
                    if (parsed.hasOwnProperty('error')) {
                        this.onError((parsed as ErrorResponse).error);
                        return;
                    } else {
                        this.onResults(parsed as ReviewItem[]);
                    }
                }
            }
        } catch (e) {
            this.onError(e.toString());
        }
    }

    cancel() {
        this.onResults = (_) => null;
        this.onError = (_) => null;
        this.onDone = () => null;
        this.abort.abort();
    }
}
