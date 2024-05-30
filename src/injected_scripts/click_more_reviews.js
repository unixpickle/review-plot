let buttons = Array.from(document.getElementsByTagName('button')).filter((x) => {
    const attr = x.getAttribute('jsaction');
    return attr && attr.endsWith('reviewChart.moreReviews');
});
if (buttons.length) {
    buttons[0].click();
    const count = parseInt(buttons[0].textContent.split(' '));
    if (isNaN(count)) {
        return 1;
    } else {
        return count;
    }
} else {
    // See if there should be a reviews button since there's a
    // "Write a review" button.
    const buttons = Array.from(document.getElementsByTagName('button'));
    if (buttons.some((x) => x.getAttribute('aria-label') == 'Write a review')) {
        return 0;
    }
    return null;
}