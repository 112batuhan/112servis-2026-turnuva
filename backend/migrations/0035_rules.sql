-- The tournament rules shown on /rules: one row of osu! BBCode, edited by hosts on /admin/rules.
-- Starts with the rules of the previous 112 tournament (its forum post), for the hosts to edit.
CREATE TABLE rules (
    id BOOLEAN PRIMARY KEY DEFAULT true CHECK (id),
    bbcode TEXT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by UUID REFERENCES osu_users(id) ON DELETE SET NULL
);

INSERT INTO rules (bbcode) VALUES ($rules$[centre][img]https://imgur-archive.ppy.sh/discord/520370557531979786-693448457154723881-botavatar.png[/img]

[url=https://osu.ppy.sh/forum/t/690315]112'nin 2018 Kış Turnuvası[/url]
[url=https://osu.ppy.sh/forum/t/755647]112'nin 2018 Yaz Turnuvası[/url]
[url=https://osu.ppy.sh/forum/t/839761]112'nin 2019 Kış Turnuvası[/url]
[url=https://osu.ppy.sh/forum/t/917454]112'nin 2019 Yaz Turnuvası[/url]

[size=150][url=https://discord.gg/g3nqqeZ]Discord[/url][/size]

[size=150][b]DISCORD SUNUCUSUNA KATILMAK ZORUNLUDUR. KAYITLAR BU SUNUCUDAKİ BİR BOT ÜZERİNDEN GERÇEKLEŞTİRİLMEKTEDİR[/b] 
Discord adınızı oyun içindeki adınıza değiştiriniz.

Turnuvada oynayacaklar kadar turnuvanın sorunsuz ilerlemesi için gönüllü çalışanlara ihtiyaç vardır. Özellikle Türk hakem eksikliğimiz var. Yardımcı olmak isteyenler bana discord sunucusundan ulaşabilir.

If you are not Turkish and still want to help me in any way, pls send me a DM on Discord. 112servis#9642[/size]

[size=150][b]Kurallar Her an değiştirilebilir. Oyuncuların kuralları bilmeleri beklenmektedir.[/b][/size][/centre]

[heading]GENEL BİLGİLER[/heading]

[list=1]
[*]Sadece türkçe konuşanlar oyuncu olarak kayıt olabilir. Türkçe konuştuğunuzu kanıtlayabilirseniz nereli olduğunuzun önemi yok. 
[*]Bu bir [b]osu!standart[/b] turnuvasıdır.
[*]Turnuvanın [b]skor sistemi v2[/b] olacaktır.
[*]Turnuva 1 hafta sıralama, 2 hafta normal maçlar oynanacak şekilde hızlı bir turnuvadır. Maçlar hafta sonu olduğu kadar hafta içi de oynanacaktır. 
[*]Turnuvaya kayıt oyuncuların ağırlıklı pp toplamları 13200 olmalıdır. Bunun hesabına buradan bakabilirsiniz: https://www.desmos.com/calculator/idwmeev5ns
[*]Turnuvada badge ödülü olmadığı için oyuncular hakemlik yapabilir. Hakem olup bize yardım etmek isteyenler discord sunucusundan hostlara ulaşabilir.
[/list]

[heading]KAYIT[/heading]

[list=1]
[*]Discord sunucusuna katılım zorunludur.  https://discord.gg/ZEDpX2F
[*]Discorddaki bota belirtilen komutlarla kayıt olun. #anonslar
[/list]

[heading]TARİHLER[/heading]

[b]Kayıt bitiş: [/b]8 Nisan 23:59

[b]İlk Maçlar:[/b] 10-11 Nisan
Diğer maçlar bu tarihlerden sonraki günlerde oynanacaktır.

[heading]SIRALAMA AŞAMASI KURALLARI[/heading]

[list=1]
[*]Sıralama aşaması poolu [b]3-2-2-2[/b] formatında olacaktır.
[*]Isınma oynanmayacaktır.
[*]No Fail zorunlu olarak her mapta olacaktır.
[*]Mapler [b]1 kere[/b] oynanacaktır. 
[*]Sıralama odalarını oyuncular seçecektir. Eğer bir oda çok boş olursa oyuncular diğer odalara yönlendirilecektir.
[*]Geç gelen oyuncular için 10dk beklenir, sonra maç başlar.
[*]Sonradan gelen oyuncular maça alınır, ama diğerlerinin kaldıkları yerden oynarlar.
[*]Odaya kimse gelmezse maç iptal edilir. Katılmayan oyuncuların puanları 0 olur.
[*]Oyuncular bütün maplerden topladıkları puanlara göre sıralanacaktır. 
[*]Eğer belirlenenden çok oyuncu olursa son sıradaki oyuncular elenecektir.
[/list]

[heading]MAÇ ZAMANLARI[/heading]

[list=1]
[*]Maç zamanları her hafta doldurulacak olan uygun zaman formuna göre düzenlenecektir. [b]Yazdığınız zamanda oynayacağınız garanti değildir.[/b] Bu sadece size yardım etme amaçlıdır. Maç zamanları sıkışık olacağı için takımlar birbiriyle önceden anlaşabilir.
[*]Belirlenen maç zamanları size uymazsa karşı takım ile anlaşıp maçın zamanını[b] maçtan 4 saat önce [/b]değiştirebilirsiniz.
[*]Eğer maça belirlenen zamanda gelemeyeceğiniz kesinse önceden bildirilmesi beklenmektedir. Bu hem karşı takımın, hem de diğer çalışanların vaktinin boşa gitmemesi için gereklidir.
[/list]

[heading]TURNUVA KURALLARI[/heading]

[list=1]
[*]Maçlar Ro16 aşamasında ve Çeyrek finalde[b] bo9[/b], Yarı finalde ve finalde [b]bo11[/b], büyük finalde [b]bo13[/b] oynanacaktır.
[*]Mappool ro16'da [b] 4-2-2-3-2-1[/b] formatında ve takım başına [b]1 ban[/b], sonraki aşamalarda [b] 5-3-3-3-3-1[/b] formatında ve takım başına[b] 2 ban[/b] olacaktır.
[*]Maça zamanında gelmeyen oyuncular/takımlar için [b]10 dk[/b] zaman tutulur. 10 dk sonunda da gelmezlerse mağlup sayılırlar.
[*]Maçlara 2 takım da gelmezse maç farklı bir zamana ertelenir ve takımlar kara listeye alınır. Kara listedeki bir takım ikinci kez aynı durumda kalırsa elenir.
[*][b]Maç zamanında bir oyuncu eksik olan takımlar mağlup sayılır[/b]
[*]Turnuva çalışanlarına ve diğer oyunculara karşı kötü davranışlarda uyarılara aldırış edilmezse oyuncular gerektiği şekilde cezalandırılabilirler.
[*]Turnuva sırasında hile yaptığı düşünülen kişiler turnuvadan uzaklaştırılacaktır.
[*]Turnuva sahibinin istediği kişiyi istediği zaman turnuvadan men etme hakkına sahiptir.
[/list]

[heading]MAÇ KURALLARI[/heading]

[list=1]
[*]Maçlar ısınma mapleriyle başlar. [b]Isınma maplerinin süresi 4 dakikayı geçmemelidir.[/b] Isınma oynamak istemeyen oyuncular/takımlar kendi ısınma haklarını atlayabilirler ama karşı takım kendi ısınma hakkını oynamak isterse oynamak zorundadırlar.
[*]Isınma oynandıktan sonra takımlardan birer kişi !roll komutu yazar. Roll kazananı İlk pick'i, ikinci banı; kaybeden takım ise ikinci pick'i, ilk banı seçer.
[*]Aynı moddan iki map banlanamaz.
[*]No Fail zorunlu olarak her mapta olacaktır.
[*]Pick ve ban sırasında her oyuncuya/takıma [b]ikişer dakika[/b] verilir. Bu süreden önce Pick veya ban belirtilmezse sıra öteki oyuncuya/takıma verilir.
[*]Map sırasında bağlantı veya hardware hatasından kaynaklı oyundan düşmelerde; ilk 30 saniye içinde yaşanırsa map tekrar oynanır, daha sonra yaşanırsa MP linkteki skor geçerlidir.
[b]Bağlantısı kopan oyuncuya geri gelmesi için 10 dakika[/b] verilir. Bu süre içinde geri gelmezse veya kopmalar sürekli oluyorsa ve her seferinde oyuncunun geri gelmesi uzun sürüyorsa maça eksik devam edilir.
[*]Map başladıktan sonra beatmap skinini veya mouse tuşlarını açık unutmak, yanlış skinle oynamak mapı tekrar oynamak için geçerli sebepler değildir. Bunlar tamamen oyuncunun sorumluluğundadır.
[*][b]Ardarda aynı mod seçilemez.(nomod mod sayılmaz, ardarda seçilebilir.)[/b]
[*]Freemod maplarında en az bir kişide mod olmalıdır.
[*]Freemod’da [b]HD, HR, EZ, FL[/b] modları alınabilir. 
[*]Eğer oyuncular/takımlar aynı skoru yaparsa hiçbir taraf puan almayacaktır.
[*]Mapın sonunda başarısız olan skorlar [b]sayılmayacaktır[/b].
[*]Tiebreaker mapı freemod’dur ama mod almak zorunlu değildir.
[/list]

[heading]ÖDÜLLER[/heading]
Kazanan takıma en az bir ay supporter hediye edilecektir. Bağışçılardan gelen supporterlara göre ödül artacaktır.$rules$);
