import { providerName } from './flows';

export const Panel = ({ title }: { title: string }) => {
  const name = providerName() ?? 'guest';
  return (
    <section className="panel">
      <h2>{title}</h2>
      <p>{name}</p>
    </section>
  );
};
