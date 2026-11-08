import { useRef } from "react";

type UncertainInsertionReviewProps = {
  onReviewed: () => void;
};

/** Requires one explicit review acknowledgment before CopyReady can be shown. */
export function UncertainInsertionReview({
  onReviewed,
}: UncertainInsertionReviewProps) {
  const acknowledged = useRef(false);

  function acknowledgeReview() {
    if (acknowledged.current) return;
    acknowledged.current = true;
    onReviewed();
  }

  return (
    <section
      aria-labelledby="uncertain-review-heading"
      className="home-status-card uncertain-review-card"
    >
      <p className="home-status-label">Manual review required</p>
      <h2 id="uncertain-review-heading">
        Check the destination before continuing
      </h2>
      <p className="home-status-description">
        Text may already have been partially inserted. Outflow will not retry
        insertion or offer Copy until you inspect the destination.
      </p>
      <button
        className="copy-result-action copy-result-action-primary"
        onClick={acknowledgeReview}
        type="button"
      >
        I have reviewed the destination
      </button>
    </section>
  );
}
