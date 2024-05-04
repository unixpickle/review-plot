interface Window {
    app: App;
}

class App {
    private locationPicker: LocationPicker;
    private search: PlaceSearch;

    constructor() {
        this.search = new PlaceSearch();
        document.body.appendChild(this.search.element());
        this.locationPicker = new LocationPicker();
        document.body.appendChild(this.locationPicker.element());
    }

    locationQueryString(): string {
        return this.locationPicker.urlQuery();
    }
}

window.app = new App();