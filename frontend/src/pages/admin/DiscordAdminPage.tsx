import { useCallback, useEffect, useState } from "react";
import { errorMessage } from "../../api/client";
import {
  deleteDiscordChannels,
  fetchDiscord,
  saveDiscordSettings,
  setDiscordActive,
  setDiscordChannelsPaused,
  syncDiscord,
} from "../../api/discord";
import type { DiscordSettings, DiscordView, GuildChannel } from "../../api/types";
import { ErrorAlert } from "../../components/ErrorAlert";
import "./DiscordAdminPage.css";

type Key = keyof DiscordSettings;
type Kind = "role" | "channel" | "category";

interface Field {
  key: Key;
  label: string;
  hint: string;
}

const SECTIONS: { title: string; kind: Kind; fields: Field[] }[] = [
  {
    title: "Roller",
    kind: "role",
    fields: [
      {
        key: "verified_role_id",
        label: "Doğrulanmış",
        hint: "Discord hesabını siteye bağlayan herkese verilir.",
      },
      {
        key: "player_role_id",
        label: "Oyuncu",
        hint: "Bir takımda olan oyunculara verilir, takım dağılınca alınır.",
      },
    ],
  },
  {
    title: "Kanallar",
    kind: "channel",
    fields: [
      {
        key: "team_announce_channel_id",
        label: "Takım duyuruları",
        hint: "Yeni takımlar. İki oyuncusu da son bir hafta içinde duyurulmuş bir takım duyurulmaz.",
      },
      {
        key: "match_announce_channel_id",
        label: "Maç duyuruları",
        hint: "Bracket maçları başlamadan 15 dakika önce, maça abone olan izleyiciler etiketlenerek.",
      },
      {
        key: "results_channel_id",
        label: "Sonuçlar",
        hint: "Biten bracket maçlarının sonuçları. Maç yeniden açılırsa mesaj silinir.",
      },
      { key: "host_channel_id", label: "Host kanalı", hint: "Olan her şeyin özeti." },
      {
        key: "referee_channel_id",
        label: "Hakem kanalı",
        hint: "Yeni saate katılamayan hakemin yerine birinin aranması ve hakemsiz yaklaşan maçlar.",
      },
      {
        key: "streamer_channel_id",
        label: "Yayıncı kanalı",
        hint: "Yeni saate katılamayan yayıncının yerine birinin aranması ve yayıncısız yaklaşan maçlar.",
      },
    ],
  },
  {
    title: "Maç kanalı kategorileri",
    kind: "category",
    fields: [
      {
        key: "qualifier_category_id",
        label: "Eleme lobileri",
        hint: "Bir lobiye ilk takım yazılınca lobinin kanalı burada açılır.",
      },
      {
        key: "qualifier_archive_category_id",
        label: "Eleme arşivi",
        hint: "Biten lobilerin salt okunur kanalları.",
      },
      {
        key: "bracket_category_id",
        label: "Bracket maçları",
        hint: "İki takımı ve saati belli olan maçın kanalı burada açılır.",
      },
      {
        key: "bracket_archive_category_id",
        label: "Bracket arşivi",
        hint: "Biten maçların salt okunur kanalları.",
      },
    ],
  },
];

/**
 * Host-only. Sets up the tournament server's bot: the roles it hands out, where it posts, and
 * where each match's private channel lives. A full category gets a second one next to it.
 */
export function DiscordAdminPage() {
  const [view, setView] = useState<DiscordView | null>(null);
  const [draft, setDraft] = useState<DiscordSettings | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [switching, setSwitching] = useState(false);
  const [channelsBusy, setChannelsBusy] = useState(false);
  const [note, setNote] = useState<string | null>(null);

  const load = useCallback(async () => {
    const v = await fetchDiscord();
    setView(v);
    setDraft(v.settings);
  }, []);

  useEffect(() => {
    load().catch((e) => setError(errorMessage(e)));
  }, [load]);

  const dirty = view && draft && JSON.stringify(view.settings) !== JSON.stringify(draft);

  async function save() {
    if (!draft) return;
    setError(null);
    setNote(null);
    setSaving(true);
    try {
      await saveDiscordSettings(draft);
      await load();
      setNote("Kaydedildi. Roller ve kanallar arka planda güncelleniyor.");
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setSaving(false);
    }
  }

  async function sync() {
    setError(null);
    setNote(null);
    try {
      await syncDiscord();
      setNote("Senkronizasyon başladı: sunucudakiler, roller ve maç kanalları kontrol ediliyor.");
    } catch (e) {
      setError(errorMessage(e));
    }
  }

  async function toggle() {
    if (!view) return;
    const active = !view.active;
    if (
      !active &&
      !window.confirm(
        "Bot kapatılsın mı? Discord bağlantısı kesilir, sırada bekleyen mesajlar silinir ve kapalıyken olanlar Discord'a gönderilmez.",
      )
    )
      return;
    setError(null);
    setNote(null);
    setSwitching(true);
    try {
      await setDiscordActive(active);
      setView({ ...view, active, connected: active && view.connected });
      setNote(
        active
          ? "Bot açıldı. Bağlanınca roller ve maç kanalları senkronize edilecek."
          : "Bot kapatıldı. Ayarları yine de düzenleyebilirsin.",
      );
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setSwitching(false);
    }
  }

  async function toggleChannels() {
    if (!view) return;
    const paused = !view.channels_paused;
    setError(null);
    setNote(null);
    setChannelsBusy(true);
    try {
      await setDiscordChannelsPaused(paused);
      setView({ ...view, channels_paused: paused });
      setNote(
        paused
          ? "Yeni maç kanalı açılması duraklatıldı. Var olan kanallar güncellenmeye devam ediyor."
          : "Maç kanalları yeniden açılıyor: kanalı olması gereken maçlar arka planda kanalını alacak.",
      );
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setChannelsBusy(false);
    }
  }

  async function deleteChannels() {
    if (!view) return;
    if (
      !window.confirm(
        view.channels_paused
          ? "Bütün maç kanalları (arşivdekiler dahil) silinsin mi? İçlerindeki mesajlar geri gelmez."
          : "Bütün maç kanalları (arşivdekiler dahil) silinsin mi? İçlerindeki mesajlar geri gelmez. Kanal açma duraklatılmadığı için kanalı olması gereken maçlara hemen yenileri açılır.",
      )
    )
      return;
    setError(null);
    setNote(null);
    setChannelsBusy(true);
    try {
      await deleteDiscordChannels();
      setNote("Maç kanalları arka planda siliniyor. Çok kanal varsa birkaç dakika sürebilir.");
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setChannelsBusy(false);
    }
  }

  return (
    <section className="panel">
      <div className="page-header">
        <h1>Discord botu</h1>
        {view &&
          (!view.enabled ? (
            <span className="badge">Kurulmadı</span>
          ) : !view.active ? (
            <span className="badge badge-danger">Kapalı</span>
          ) : view.connected ? (
            <span className="badge badge-success">Bağlı</span>
          ) : (
            <span className="badge badge-warning">Bağlanmadı</span>
          ))}
        {view?.enabled && (
          <div className="page-header-aside">
            {view.active && (
              <button type="button" className="btn btn-secondary btn-sm" onClick={sync}>
                Şimdi senkronize et
              </button>
            )}
            <button
              type="button"
              className={`btn btn-sm ${view.active ? "btn-danger" : "btn-primary"}`}
              disabled={switching}
              onClick={toggle}
            >
              {view.active ? "Botu kapat" : "Botu aç"}
            </button>
          </div>
        )}
      </div>

      <ErrorAlert message={error} onDismiss={() => setError(null)} />
      {note && <p className="status discord-note">{note}</p>}
      {!view && !error && <p className="status">Yükleniyor…</p>}

      {view && !view.enabled && (
        <p className="status">
          Bot kurulmadı. Kurmak için sunucuda <code>DISCORD_BOT_TOKEN</code> ve{" "}
          <code>DISCORD_GUILD_ID</code> ayarlanmalı (ayrıntılar <code>.env.example</code>{" "}
          dosyasında).
        </p>
      )}
      {view?.enabled && !view.active && (
        <p className="status discord-note">
          Bot kapalı: Discord'a bağlı değil, hiçbir şey göndermiyor ve kapalıyken olanları sıraya
          almıyor. Açınca sunucudakileri, rolleri ve maç kanallarını baştan senkronize eder.
          Kapalıyken olanların mesajları gönderilmez; saati henüz geçmemiş maç hatırlatmaları ve
          duyuruları gider.
        </p>
      )}
      {view?.enabled && !view.guild && (
        <p className="status discord-note">
          Discord sunucusu okunamadı, ID'leri elle girebilirsin. Botun sunucuda olduğundan emin ol.
        </p>
      )}

      {view?.enabled && (
        <div className="discord-channels">
          <div className="discord-channels-text">
            <span className="discord-label">Maç kanalları</span>
            {view.channels_paused ? (
              <span className="badge badge-warning">Açma duraklatıldı</span>
            ) : (
              <span className="badge badge-success">Açılıyor</span>
            )}
            <span className="muted">{view.channel_count} maçın kanalı var</span>
          </div>
          <div className="discord-actions">
            <button
              type="button"
              className="btn btn-secondary btn-sm"
              disabled={channelsBusy}
              onClick={toggleChannels}
            >
              {view.channels_paused ? "Kanal açmayı sürdür" : "Kanal açmayı duraklat"}
            </button>
            <button
              type="button"
              className="btn btn-danger btn-sm"
              disabled={channelsBusy || !view.active}
              title={view.active ? undefined : "Bot kapalıyken kanallar silinemez"}
              onClick={deleteChannels}
            >
              Tüm maç kanallarını sil
            </button>
          </div>
        </div>
      )}

      {view?.enabled && draft && (
        <form
          className="discord-form"
          onSubmit={(e) => {
            e.preventDefault();
            save();
          }}
        >
          {SECTIONS.map((section) => (
            <fieldset key={section.title} className="discord-section">
              <legend className="discord-section-title">{section.title}</legend>
              {section.fields.map((field) => (
                <div key={field.key} className="discord-field">
                  <label className="discord-label" htmlFor={`discord-${field.key}`}>
                    {field.label}
                  </label>
                  <Picker
                    id={`discord-${field.key}`}
                    kind={section.kind}
                    view={view}
                    value={draft[field.key]}
                    onChange={(value) => setDraft({ ...draft, [field.key]: value })}
                  />
                  <p className="discord-hint muted">{field.hint}</p>
                </div>
              ))}
            </fieldset>
          ))}
          <div className="discord-actions">
            <button type="submit" className="btn btn-primary" disabled={!dirty || saving}>
              {saving ? "Kaydediliyor…" : "Kaydet"}
            </button>
            {dirty && (
              <button
                type="button"
                className="btn btn-secondary"
                disabled={saving}
                onClick={() => setDraft(view.settings)}
              >
                Vazgeç
              </button>
            )}
          </div>
        </form>
      )}
    </section>
  );
}

interface PickerProps {
  id: string;
  kind: Kind;
  view: DiscordView;
  value: string | null;
  onChange: (value: string | null) => void;
}

/** A select of the server's roles, channels or categories, or an id field when they can't be read. */
function Picker({ id, kind, view, value, onChange }: PickerProps) {
  const guild = view.guild;
  if (!guild) {
    return (
      <input
        id={id}
        className="input"
        inputMode="numeric"
        placeholder="ID"
        value={value ?? ""}
        onChange={(e) => onChange(e.target.value.trim() || null)}
      />
    );
  }

  const known =
    kind === "role"
      ? guild.roles.some((r) => r.id === value)
      : kind === "category"
        ? guild.categories.some((c) => c.id === value)
        : guild.text_channels.some((c) => c.id === value);

  return (
    <select
      id={id}
      className="select"
      value={value ?? ""}
      onChange={(e) => onChange(e.target.value || null)}
    >
      <option value="">Seçilmedi</option>
      {value && !known && <option value={value}>Bulunamadı ({value})</option>}
      {kind === "role" &&
        guild.roles.map((r) => (
          <option key={r.id} value={r.id}>
            {r.name}
          </option>
        ))}
      {kind === "category" &&
        guild.categories.map((c) => (
          <option key={c.id} value={c.id}>
            {c.name}
          </option>
        ))}
      {kind === "channel" && (
        <ChannelOptions channels={guild.text_channels} categories={guild.categories} />
      )}
    </select>
  );
}

/** Text channels grouped under their categories, uncategorised ones first. */
function ChannelOptions({
  channels,
  categories,
}: {
  channels: GuildChannel[];
  categories: GuildChannel[];
}) {
  const loose = channels.filter(
    (c) => !c.parent_id || !categories.some((cat) => cat.id === c.parent_id),
  );
  const option = (c: GuildChannel) => (
    <option key={c.id} value={c.id}>
      #{c.name}
    </option>
  );
  return (
    <>
      {loose.map(option)}
      {categories.map((cat) => {
        const inside = channels.filter((c) => c.parent_id === cat.id);
        return inside.length === 0 ? null : (
          <optgroup key={cat.id} label={cat.name}>
            {inside.map(option)}
          </optgroup>
        );
      })}
    </>
  );
}
