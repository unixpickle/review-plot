interface Window {
    app: App;
}

class App {
    private locationPicker: LocationPicker;
    private search: PlaceSearch;

    constructor() {
        this.locationPicker = new LocationPicker();
        document.body.appendChild(this.locationPicker.element());
        this.search = new PlaceSearch();
        document.body.appendChild(this.search.element());
    }

    urlEncodeLocation(): string {
        return this.locationPicker.urlEncode();
    }
}

window.app = new App();