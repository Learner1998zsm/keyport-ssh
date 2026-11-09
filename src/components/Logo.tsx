export function LogoMark({ size = 42 }: { size?: number }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 48 48"
      fill="none"
      aria-hidden="true"
      className="logo-mark"
    >
      <rect x="1.25" y="1.25" width="45.5" height="45.5" rx="13.5" stroke="currentColor" strokeOpacity=".32" />
      <circle cx="24" cy="24" r="13.2" stroke="currentColor" strokeOpacity=".22" strokeDasharray="1.5 4" />
      <path d="M24 5.5v3M24 39.5v3M5.5 24h3M39.5 24h3" stroke="currentColor" strokeWidth="1.3" strokeLinecap="round" />
      <circle cx="24" cy="20" r="6.1" stroke="#f26a3d" strokeWidth="2.1" />
      <path d="M24 26v11.2M20.3 35.5h7.4" stroke="currentColor" strokeWidth="2.1" strokeLinecap="round" />
      <circle cx="24" cy="20" r="2" fill="#f26a3d" />
    </svg>
  );
}

export function BrandLockup() {
  return (
    <div className="brand-lockup">
      <LogoMark />
      <div>
        <span className="brand-lockup__word">KEYPORT</span>
        <span className="brand-lockup__sub">SSH KEY CONCIERGE</span>
      </div>
    </div>
  );
}
