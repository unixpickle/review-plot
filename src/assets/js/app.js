class App {
    constructor() {
        this.search = new PlaceSearch();
        document.body.appendChild(this.search.element());
    }
}
window.app = new App();
//# sourceMappingURL=app.js.map