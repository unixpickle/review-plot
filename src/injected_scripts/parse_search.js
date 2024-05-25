const divs = document.getElementsByTagName('div');
const results = [];
for (let i = 0; i < divs.length; i++) {
    const div = divs[i];
    if ((div.getAttribute('aria-label') || '').startsWith('Results for')) {
        const links = div.getElementsByTagName('a');
        for (let j = 0; j < links.length; j++) {
            const link = links[j];
            const href = link.href;
            const name = link.getAttribute('aria-label');
            if (href && name && href.startsWith('https://www.google.com/maps/place')) {
                const lines = [];
                const parent = link.parentElement;

                // Skip listings which are ads.
                const h1s = Array.from(parent.getElementsByTagName('h1'));
                if (h1s.some((x) => x.getAttribute('aria-label') == 'Sponsored')) {
                    continue;
                }

                const extension = parent.getElementsByClassName('section-subtitle-extension');
                for (let i = 0; i < extension.length; i++) {
                    let sibling = extension[i].nextSibling;
                    while (sibling) {
                        const spans = sibling.getElementsByTagName('span');
                        for (let j = 0; j < spans.length; j++) {
                            const span = spans[j];

                            // Skip the hidden/image children of the reviews span.
                            if (span.getAttribute('aria-hidden')) {
                                continue;
                            }

                            // Include reviews (count and stars) if possible.
                            if (span.getAttribute('role') == 'img') {
                                const label = span.getAttribute('aria-label');
                                if (label.toLowerCase().includes('star')) {
                                    lines.push(label);
                                    continue;
                                }
                            }

                            // Skip parent spans which contain children.
                            if (span.getElementsByTagName('span').length) {
                                continue;
                            }

                            const text = span.textContent;
                            if (text.length > 1) {
                                if (text.startsWith(' ⋅ ') && lines.length) {
                                    lines[lines.length - 1] += text;
                                } else {
                                    lines.push(text);
                                }
                            }
                        }
                        sibling = sibling.nextSibling;
                    }
                }
                results.push({ name: name, url: href, extra: lines });
            }
        }
    }
}
return results;