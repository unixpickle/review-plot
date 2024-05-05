class App {
    constructor() {
        this.locationPicker = new LocationPicker();
        document.body.appendChild(this.locationPicker.element());
        this.search = new PlaceSearch();
        document.body.appendChild(this.search.element());
        this.plot = new ReviewPlot();
        document.body.appendChild(this.plot.element());
    }
    urlEncodeLocation() {
        return this.locationPicker.urlEncode();
    }
}
window.app = new App();
//# sourceMappingURL=app.js.map