class App {
    constructor() {
        this.search = new PlaceSearch();
        document.body.appendChild(this.search.element());
        this.locationPicker = new LocationPicker();
        document.body.appendChild(this.locationPicker.element());
    }
    locationQueryString() {
        return this.locationPicker.urlQuery();
    }
}
window.app = new App();
//# sourceMappingURL=app.js.map