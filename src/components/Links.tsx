const LINKS = [
  ["GitHub", "https://github.com/nova-wallet/metadata-portal"],
  ["Terms of Service", "https://novawallet.io/terms"],
];

export const Links = () => {
  return (
    <div className="flex space-x-2 text-black opacity-70">
      {LINKS.map(([label, href], i) => (
        <a
          className="bordered-action hover:bg-neutral-100 transition-colors"
          href={href}
          key={i}
        >
          {label}
        </a>
      ))}
    </div>
  );
};