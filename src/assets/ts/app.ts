interface Window {
    app: App;
}

class App {
    private search: PlaceSearch;

    constructor() {
        this.search = new PlaceSearch();
        document.body.appendChild(this.search.element());
    }
}

window.app = new App();