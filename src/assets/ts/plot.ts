interface ReviewItem {
    timestamp: number;
    author: string;
    content: string;
    rating: number;
}

interface ReviewAverage {
    timestamp: number;
    rating: number;
}

class ReviewPlot {
    private items: ReviewItem[] = [];

    private _element: HTMLElement;
    private startDate: HTMLLabelElement;
    private endDate: HTMLLabelElement;
    private graph: SVGSVGElement;

    private loader: HTMLElement;

    private statusName: HTMLLabelElement;
    private statusCount: HTMLLabelElement;
    private statusFit: HTMLLabelElement;
    private statusError: HTMLLabelElement;

    private cancelButton: HTMLButtonElement;
    private downloadButton: HTMLButtonElement;
    private granularity: HTMLInputElement;
    private dateRange: HTMLSelectElement;

    private query: ReviewQuery;

    constructor() {
        this._element = document.createElement('div');
        this._element.className = 'plot plot-empty';
        this.startDate = document.createElement('label');
        this.startDate.className = 'plot-start-date';
        this.endDate = document.createElement('label');
        this.endDate.className = 'plot-end-date';
        this.graph = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
        this.graph.setAttribute('class', 'plot-graph');
        this.graph.setAttribute('viewBox', '0 0 440 300');

        this.loader = document.createElement('div');
        this.loader.className = 'loader';

        const status = document.createElement('div');
        status.className = 'plot-status';
        this.statusName = document.createElement('label');
        this.statusName.textContent = 'No location selected';
        this.statusName.className = 'plot-status-name';
        this.statusCount = document.createElement('label');
        this.statusCount.className = 'plot-status-count';
        this.statusFit = document.createElement('label');
        this.statusFit.className = 'plot-status-fit';
        this.statusError = document.createElement('label');
        this.statusError.className = 'plot-status-error';
        status.appendChild(this.statusName);
        status.appendChild(this.statusCount);
        status.appendChild(this.statusFit);
        status.appendChild(this.statusError);

        const controls = document.createElement('div');
        controls.className = 'plot-controls';
        this.cancelButton = createControlsButton(controls, 'Cancel');
        this.cancelButton.classList.add('plot-controls-cancel');
        this.cancelButton.addEventListener('click', () => this.cancel());
        this.downloadButton = createControlsButton(controls, 'Download');
        this.downloadButton.classList.add('plot-controls-download');
        this.downloadButton.addEventListener('click', () => this.download());
        this.granularity = createControlsInput(controls, 'Granularity');
        this.granularity.type = 'range';
        this.granularity.min = '5';
        this.granularity.max = '100';
        this.granularity.value = '20';
        this.granularity.addEventListener('input', () => this.updateUI());
        this.dateRange = createControlsSelect(controls, 'Date range', [
            ['All time', 'all'],
            ['5 years', '5 years'],
            ['1 year', '1 year'],
            ['6 months', '6 months'],
        ]);
        this.dateRange.addEventListener('change', () => this.updateUI());

        this._element.appendChild(this.startDate);
        this._element.appendChild(this.endDate);
        this._element.appendChild(this.graph);
        this._element.appendChild(this.loader);
        this._element.appendChild(status);
        this._element.appendChild(controls);

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
        };
        this.query.run();
    }

    private cancel() {
        if (!this.query) {
            return;
        }
        this.query.cancel();
        this.query = null;
        this.setStatus('loaded');
        this.statusCount.textContent = `${this.items.length} results (stopped)`;
    }

    private download() {
        let data = (
            'timestamp,rating,author,content\n' +
            this.items.map((x) => {
                const fields = [
                    Math.round(x.timestamp).toString(), x.rating.toFixed(1), x.author, x.content,
                ];
                return fields.map(escapeCSVField).join(',');
            }).join('\n')
        );
        const blob = new Blob([data], { type: 'text/csv' });
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = 'data.csv';
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
        URL.revokeObjectURL(url);
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
        this.graph.textContent = '';

        const [items, fit] = this.averagedAndFilteredItems();

        if (items.length == 0) {
            this.startDate.textContent = 'No data';
            this.endDate.textContent = 'No data';
            this.statusFit.classList.remove('plot-status-fit-available');
            return;
        }

        this.statusFit.textContent = `Fit: ${fit.slope.toFixed(3)}*x + ${fit.bias.toFixed(3)}`;
        this.statusFit.classList.add('plot-status-fit-available');

        const start = new Date(items[0].timestamp * 1000);
        const end = new Date(items[items.length - 1].timestamp * 1000);
        this.startDate.textContent = formatDate(start);
        this.endDate.textContent = formatDate(end);

        const span = Math.max(1, items[items.length - 1].timestamp - items[0].timestamp);

        items.forEach((x) => {
            const timestamp = x.timestamp;
            const frac = items.length > 1 ? (timestamp - items[0].timestamp) / span : 0.5;
            const dot = document.createElementNS('http://www.w3.org/2000/svg', 'circle');
            dot.setAttribute('class', 'plot-graph-dot');
            dot.setAttribute('r', '10');
            dot.setAttribute('fill', '#65bcd4');
            dot.setAttribute('cx', marginPercent(frac));
            dot.setAttribute('cy', marginPercent(1 - (x.rating - 1) / 4));
            this.graph.appendChild(dot);
        });

        if (items.length > 1) {
            // 5% through 95% is actually 0 and 1.
            // This means we want to evaluate the line at
            // -5/(90) and 95/90.
            const y1 = fit.bias + fit.slope * (-5 / 90);
            const y2 = fit.bias + fit.slope * 95 / 90;
            const createLine = () => {
                const fitLine = document.createElementNS('http://www.w3.org/2000/svg', 'line');
                fitLine.setAttribute('x1', '0%');
                fitLine.setAttribute('x2', '100%');
                fitLine.setAttribute('y1', marginPercent(1 - (y1 - 1) / 4));
                fitLine.setAttribute('y2', marginPercent(1 - (y2 - 1) / 4));
                fitLine.setAttribute('stroke', '#65bcd4');
                fitLine.setAttribute('stroke-width', '4');
                return fitLine;
            }
            const bg = createLine();
            bg.setAttribute('stroke', '#000');
            bg.setAttribute('stroke-width', '5');
            this.graph.appendChild(bg);
            this.graph.appendChild(createLine());
        }
    }

    private firstAllowedTimestamp(): number {
        const val = this.dateRange.value;
        if (val == 'all') {
            return 0;
        }
        if (val.endsWith('years') || val.endsWith('year')) {
            const now = new Date();
            now.setFullYear(now.getFullYear() - parseFloat(val.split(' ')[0]));
            return now.getTime() / 1000;
        }
        if (val.endsWith('months') || val.endsWith('month')) {
            const now = new Date();
            now.setMonth(now.getMonth() - parseFloat(val.split(' ')[0]));
            return now.getTime() / 1000;
        }
        throw new Error(`unexpected date range: ${val}`);
    }

    private averagedAndFilteredItems(): [ReviewAverage[], LinearFit] {
        const minTime = this.firstAllowedTimestamp();
        const items = this.items.filter((x) => x.timestamp >= minTime);

        let fit = { bias: 2.5, slope: 0 };

        if (items.length < 2) {
            return [items, fit];
        }

        const start = items[0].timestamp;
        const end = items[items.length - 1].timestamp;
        const span = end - start;

        if (span == 0) {
            return [items, fit];
        }

        const xs = items.map((x) => (x.timestamp - start) / Math.max(1e-8, span));
        const ys = items.map((x) => x.rating);
        fit = linearFit(xs, ys);

        const numWindows = parseInt(this.granularity.value);
        const windowSize = span / numWindows;
        const windowItems: ReviewItem[][] = [];
        for (let i = 0; i < numWindows; i++) {
            windowItems.push([]);
        }
        items.forEach((item) => {
            const window = Math.min(
                numWindows - 1,
                Math.floor((item.timestamp - start) / windowSize),
            );
            windowItems[window].push(item);
        });

        const result: ReviewAverage[] = [];
        windowItems.forEach((items) => {
            if (items.length == 0) {
                // No reviews in this bucket.
                return;
            }
            let ratingSum = 0.0;
            let timestampSum = 0.0;
            items.forEach((x) => {
                ratingSum += x.rating;
                timestampSum += x.timestamp;
            });
            result.push({
                timestamp: timestampSum / items.length,
                rating: ratingSum / items.length,
            });
        });
        return [result, fit];
    }
}

function createControlsInput(container: HTMLElement, name: string): HTMLInputElement {
    const field = document.createElement('div');
    field.className = 'plot-controls-field plot-controls-input-field';
    const label = document.createElement('label');
    label.textContent = name;
    const input = document.createElement('input');
    field.appendChild(label);
    field.appendChild(input);
    container.appendChild(field);
    return input;
}

function createControlsButton(container: HTMLElement, value: string): HTMLButtonElement {
    const field = document.createElement('div');
    field.className = 'plot-controls-field plot-controls-button-field';
    const button = document.createElement('button');
    button.textContent = value;
    field.appendChild(button);
    container.appendChild(field);
    return button;
}

function createControlsSelect(
    container: HTMLElement,
    name: string,
    namesAndValues: [string, string][],
): HTMLSelectElement {
    const field = document.createElement('div');
    field.className = 'plot-controls-field plot-controls-select-field';
    const label = document.createElement('label');
    label.textContent = name;
    const select = document.createElement('select');
    namesAndValues.forEach((nameAndValue) => {
        const option = document.createElement('option');
        option.value = nameAndValue[1];
        option.textContent = nameAndValue[0];
        select.appendChild(option);
    });
    select.value = namesAndValues[0][1];
    field.appendChild(label);
    field.appendChild(select);
    container.appendChild(field);
    return select;
}

function formatDate(date: Date): string {
    let month = (date.getMonth() + 1).toString();
    let day = date.getDate().toString();
    var year = date.getFullYear().toString();
    month = (month.length == 1) ? '0' + month : month;
    day = (day.length == 1) ? '0' + day : day;
    return `${month}/${day}/${year}`;
}

function escapeCSVField(str: string) {
    if (str.includes(',') || str.includes('"') || str.includes('\n') || str.includes('\r')) {
        const escapedStr = str.replace('"', '""');
        return `"${escapedStr}"`;
    }
    return str;
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
            const results = await fetch(url, { signal: this.abort.signal });
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
        } finally {
            if (this.abort !== null) {
                this.abort.abort();
            }
        }
    }

    cancel() {
        this.onResults = (_) => null;
        this.onError = (_) => null;
        this.onDone = () => null;
        this.abort.abort();
        this.abort = null;
    }
}
