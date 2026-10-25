import type { Attachment } from "svelte/attachments";

/**
 * Fades an element in when it enters the screen: `{@attach reveal()}`.
 * The element stays visible when JavaScript is disabled, since the attribute is only set here.
 */
export function reveal(delay = 0): Attachment<HTMLElement> {
	return (element) => {
		element.dataset.reveal = "hidden";
		element.style.setProperty("--reveal-delay", `${delay}ms`);

		const observer = new IntersectionObserver(
			(entries) => {
				for (const entry of entries) {
					if (entry.isIntersecting) {
						element.dataset.reveal = "visible";
						observer.disconnect();
					}
				}
			},
			{ rootMargin: "0px 0px -10% 0px" },
		);
		observer.observe(element);

		return () => observer.disconnect();
	};
}
