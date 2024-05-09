interface Window {
    app: App;
}

class App {
    private locationPicker: LocationPicker;
    private search: PlaceSearch;
    private plot: ReviewPlot;

    constructor() {
        this.locationPicker = new LocationPicker();
        document.body.appendChild(this.locationPicker.element());
        this.search = new PlaceSearch();
        document.body.appendChild(this.search.element());
        this.plot = new ReviewPlot();
        document.body.appendChild(this.plot.element());

        this.search.onResult = (result) => {
            this.plot.startQuery(result.name, result.url);
        };
    }

    urlEncodeLocation(): string {
        return this.locationPicker.urlEncode();
    }
}

window.app = new App();