class App {
    constructor() {
        const searchContainer = document.createElement('div');
        searchContainer.className = 'search-container';
        this.searchBox = document.createElement('input');
        this.searchBox.placeholder = 'Name of business';
        this.searchButton = document.createElement('button');
        this.searchButton.textContent = 'Search';
        searchContainer.appendChild(this.searchBox);
        searchContainer.appendChild(this.searchButton);
        document.body.appendChild(searchContainer);
    }
}
window.app = new App();
//# sourceMappingURL=app.js.map