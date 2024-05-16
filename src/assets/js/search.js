var __awaiter = (this && this.__awaiter) || function (thisArg, _arguments, P, generator) {
    function adopt(value) { return value instanceof P ? value : new P(function (resolve) { resolve(value); }); }
    return new (P || (P = Promise))(function (resolve, reject) {
        function fulfilled(value) { try { step(generator.next(value)); } catch (e) { reject(e); } }
        function rejected(value) { try { step(generator["throw"](value)); } catch (e) { reject(e); } }
        function step(result) { result.done ? resolve(result.value) : adopt(result.value).then(fulfilled, rejected); }
        step((generator = generator.apply(thisArg, _arguments || [])).next());
    });
};
class PlaceSearch {
    constructor() {
        this.currentAbort = null;
        this._element = document.createElement('div');
        this._element.className = 'search';
        const searchBar = document.createElement('div');
        searchBar.className = 'search-bar';
        this.searchBox = document.createElement('input');
        this.searchBox.placeholder = 'Name of business';
        this.searchButton = document.createElement('button');
        this.searchButton.textContent = '🔎';
        this.searchBox.addEventListener('keyup', (e) => {
            if (e.key === 'Enter' || e.keyCode === 13) {
                this.lookupResults();
            }
        });
        this.searchButton.addEventListener('click', () => this.lookupResults());
        searchBar.appendChild(this.searchBox);
        searchBar.appendChild(this.searchButton);
        this._element.appendChild(searchBar);
        this.searchResults = document.createElement('div');
        this.searchResults.className = 'search-results search-results-state-no-results';
        this.noResults = document.createElement('div');
        this.noResults.className = 'search-results-none';
        this.noResults.textContent = PlaceSearch.HELP_STRING;
        this.errorMessage = document.createElement('div');
        this.errorMessage.className = 'search-results-error';
        this.resultItems = document.createElement('div');
        this.resultItems.className = 'search-results-items';
        this.searchResults.appendChild(this.noResults);
        this.searchResults.appendChild(this.errorMessage);
        this.searchResults.appendChild(this.resultItems);
        const loader = document.createElement('div');
        loader.className = 'loader';
        this.searchResults.appendChild(loader);
        this._element.appendChild(this.searchResults);
    }
    element() {
        return this._element;
    }
    lookupResults() {
        return __awaiter(this, void 0, void 0, function* () {
            if (this.currentAbort !== null) {
                this.currentAbort.abort();
                this.currentAbort = null;
            }
            const query = this.searchBox.value.trim();
            if (!query) {
                this.noResults.textContent = PlaceSearch.HELP_STRING;
                this.setState('no-results');
                return;
            }
            this.currentAbort = new AbortController();
            this.setState('loading');
            const location = window.app.urlEncodeLocation();
            const url = `/api/search?query=${encodeURIComponent(query)}&${location}`;
            try {
                const result = yield (yield fetch(url, { signal: this.currentAbort.signal })).json();
                this.currentAbort = null;
                if (result instanceof Array) {
                    if (result.length) {
                        this.showResults(result);
                    }
                    else {
                        this.setState('no-results');
                        this.noResults.textContent = `No results for: ${query}`;
                    }
                }
                else if (result.hasOwnProperty('error')) {
                    throw result['error'];
                }
            }
            catch (e) {
                if (e instanceof DOMException && e.name == 'AbortError') {
                    return;
                }
                this.currentAbort = null;
                this.showError(e.toString());
            }
        });
    }
    showError(e) {
        this.setState('failed');
        this.errorMessage.textContent = e;
    }
    showResults(results) {
        this.setState('some-results');
        this.resultItems.textContent = '';
        results.forEach((x) => {
            const result = document.createElement('div');
            result.className = 'search-result';
            const name = document.createElement('label');
            name.textContent = x.name;
            result.appendChild(name);
            x.extra.forEach((content) => {
                const line = document.createElement('span');
                line.className = 'search-result-extra';
                line.textContent = content;
                result.appendChild(line);
            });
            result.addEventListener('click', () => this.onResult(x));
            this.resultItems.appendChild(result);
        });
    }
    setState(state) {
        this.searchResults.className = 'search-results';
        this.searchResults.classList.add(`search-results-state-${state}`);
    }
}
PlaceSearch.HELP_STRING = 'Search for locations to see them here.';
//# sourceMappingURL=search.js.map